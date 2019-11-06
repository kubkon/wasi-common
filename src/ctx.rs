use crate::fdentry::FdEntry;
use crate::{wasi, Error, Result};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::env;
use std::ffi::{CString, OsString};
use std::fs::File;
use std::path::{Path, PathBuf};

enum PendingFdEntry {
    Thunk(fn() -> Result<FdEntry>),
    File(File),
}

impl std::fmt::Debug for PendingFdEntry {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PendingFdEntry::Thunk(f) => write!(
                fmt,
                "PendingFdEntry::Thunk({:p})",
                f as *const fn() -> Result<FdEntry>
            ),
            PendingFdEntry::File(f) => write!(fmt, "PendingFdEntry::File({:?})", f),
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
enum PendingCString {
    Bytes(Vec<u8>),
    OsString(OsString),
}

impl From<Vec<u8>> for PendingCString {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes)
    }
}

impl From<OsString> for PendingCString {
    fn from(s: OsString) -> Self {
        Self::OsString(s)
    }
}

impl PendingCString {
    #[cfg(not(windows))]
    fn into_bytes(self) -> Result<Vec<u8>> {
        use std::os::unix::ffi::OsStringExt;
        match self {
            PendingCString::Bytes(v) => Ok(v),
            // on Unix, we use the bytes from the CString directly
            PendingCString::OsString(s) => Ok(s.into_vec()),
        }
    }

    #[cfg(windows)]
    fn into_bytes(self) -> Result<Vec<u8>> {
        match self {
            PendingCString::Bytes(v) => Ok(v),
            // on Windows, we go through conversion into a `String` in order to get bytes
            PendingCString::OsString(s) => s.into_string().map(|s| s.into_bytes()),
        }
        .map_err(|_| Error::ENOTCAPABLE)
    }

    fn into_cstring(self) -> Result<CString> {
        self.into_bytes()
            .and_then(|v| CString::new(v).map_err(|_| Error::ENOTCAPABLE))
    }
}

/// A builder allowing customizable construction of `WasiCtx` instances.
pub struct WasiCtxBuilder {
    fds: HashMap<wasi::__wasi_fd_t, PendingFdEntry>,
    preopens: Vec<(PathBuf, File)>,
    args: Vec<PendingCString>,
    env: HashMap<PendingCString, PendingCString>,
}

impl WasiCtxBuilder {
    /// Builder for a new `WasiCtx`.
    pub fn new() -> Result<Self> {
        let mut builder = Self {
            fds: HashMap::new(),
            preopens: Vec::new(),
            args: vec![],
            env: HashMap::new(),
        };

        builder.fds.insert(0, PendingFdEntry::Thunk(FdEntry::null));
        builder.fds.insert(1, PendingFdEntry::Thunk(FdEntry::null));
        builder.fds.insert(2, PendingFdEntry::Thunk(FdEntry::null));

        Ok(builder)
    }

    /// Add arguments to the command-line arguments list.
    ///
    /// Arguments must not contain NUL bytes, or `WasiCtxBuilder::build()` will fail with
    /// `Error::ENOTCAPABLE`.
    pub fn args<S: AsRef<[u8]>>(mut self, args: impl Iterator<Item = S>) -> Result<Self> {
        self.args = args.map(|arg| arg.as_ref().to_vec().into()).collect();
        Ok(self)
    }

    /// Add an argument to the command-line arguments list.
    ///
    /// Arguments must not contain NUL bytes, or `WasiCtxBuilder::build()` will fail with
    /// `Error::ENOTCAPABLE`.
    pub fn arg<S: AsRef<[u8]>>(mut self, arg: S) -> Result<Self> {
        self.args.push(arg.as_ref().to_vec().into());
        Ok(self)
    }

    /// Inherit the command-line arguments from the host process.
    pub fn inherit_args(mut self) -> Result<Self> {
        self.args = env::args_os().map(PendingCString::OsString).collect();
        Ok(self)
    }

    /// Inherit the stdin, stdout, and stderr streams from the host process.
    pub fn inherit_stdio(mut self) -> Result<Self> {
        self.fds
            .insert(0, PendingFdEntry::Thunk(FdEntry::duplicate_stdin));
        self.fds
            .insert(1, PendingFdEntry::Thunk(FdEntry::duplicate_stdout));
        self.fds
            .insert(2, PendingFdEntry::Thunk(FdEntry::duplicate_stderr));
        Ok(self)
    }

    /// Inherit the environment variables from the host process.
    pub fn inherit_env(mut self) -> Result<Self> {
        self.env = std::env::vars_os()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        Ok(self)
    }

    /// Add an entry to the environment.
    ///
    /// Environment variable keys and values must not contain NUL bytes, or
    /// `WasiCtxBuilder::build()` will fail with `Error::ENOTCAPABLE`.
    pub fn env<S: AsRef<[u8]>>(mut self, k: S, v: S) -> Result<Self> {
        self.env
            .insert(k.as_ref().to_vec().into(), v.as_ref().to_vec().into());
        Ok(self)
    }

    /// Add entries to the environment.
    ///
    /// Environment variable keys and values must not contain NUL bytes, or
    /// `WasiCtxBuilder::build()` will fail with `Error::ENOTCAPABLE`.
    pub fn envs<S: AsRef<[u8]>, T: Borrow<(S, S)>>(
        mut self,
        envs: impl Iterator<Item = T>,
    ) -> Result<Self> {
        self.env = envs
            .map(|t| {
                let (k, v) = t.borrow();
                (k.as_ref().to_vec().into(), v.as_ref().to_vec().into())
            })
            .collect();
        Ok(self)
    }

    /// Provide a File to use as stdin
    pub fn stdin(mut self, file: File) -> Result<Self> {
        self.fds.insert(0, PendingFdEntry::File(file));
        Ok(self)
    }

    /// Provide a File to use as stdout
    pub fn stdout(mut self, file: File) -> Result<Self> {
        self.fds.insert(1, PendingFdEntry::File(file));
        Ok(self)
    }

    /// Provide a File to use as stderr
    pub fn stderr(mut self, file: File) -> Result<Self> {
        self.fds.insert(2, PendingFdEntry::File(file));
        Ok(self)
    }

    /// Add a preopened directory.
    pub fn preopened_dir<P: AsRef<Path>>(mut self, dir: File, guest_path: P) -> Self {
        self.preopens.push((guest_path.as_ref().to_owned(), dir));
        self
    }

    /// Build a `WasiCtx`, consuming this `WasiCtxBuilder`.
    pub fn build(self) -> Result<WasiCtx> {
        // process arguments and environment variables into `CString`s, failing quickly if they
        // contain any NUL bytes, or if conversion from `OsString` fails
        let args = self
            .args
            .into_iter()
            .map(|arg| arg.into_cstring())
            .collect::<Result<Vec<CString>>>()?;

        let env = self
            .env
            .into_iter()
            .map(|(k, v)| {
                k.into_bytes().and_then(|mut pair| {
                    v.into_bytes().and_then(|mut v| {
                        pair.push(b'=');
                        pair.append(&mut v);
                        pair.push(b'\0');
                        // we've added the nul byte at the end, but the keys and values have not yet been
                        // checked for nuls, so we do a final check here
                        CString::new(pair).map_err(|_| Error::ENOTCAPABLE)
                    })
                })
            })
            .collect::<Result<Vec<CString>>>()?;

        let mut fds: HashMap<wasi::__wasi_fd_t, FdEntry> = HashMap::new();
        // populate the non-preopen fds
        for (fd, pending) in self.fds {
            log::debug!("WasiCtx inserting ({:?}, {:?})", fd, pending);
            match pending {
                PendingFdEntry::Thunk(f) => {
                    fds.insert(fd, f()?);
                }
                PendingFdEntry::File(f) => {
                    fds.insert(fd, FdEntry::from(f)?);
                }
            }
        }
        // then add the preopen fds. startup code in the guest starts looking at fd 3 for preopens,
        // so we start from there. this variable is initially 2, though, because the loop
        // immediately does the increment and check for overflow
        let mut preopen_fd: wasi::__wasi_fd_t = 2;
        for (guest_path, dir) in self.preopens {
            // we do the increment at the beginning so that we don't overflow unnecessarily if we
            // have exactly the maximum number of file descriptors
            preopen_fd = preopen_fd.checked_add(1).ok_or(Error::ENFILE)?;

            if !dir.metadata()?.is_dir() {
                return Err(Error::EBADF);
            }

            // we don't currently allow setting file descriptors other than 0-2, but this will avoid
            // collisions if we restore that functionality in the future
            while fds.contains_key(&preopen_fd) {
                preopen_fd = preopen_fd.checked_add(1).ok_or(Error::ENFILE)?;
            }
            let mut fe = FdEntry::from(dir)?;
            fe.preopen_path = Some(guest_path);
            log::debug!("WasiCtx inserting ({:?}, {:?})", preopen_fd, fe);
            fds.insert(preopen_fd, fe);
            log::debug!("WasiCtx fds = {:?}", fds);
        }

        Ok(WasiCtx { args, env, fds })
    }
}

#[derive(Debug)]
pub struct WasiCtx {
    fds: HashMap<wasi::__wasi_fd_t, FdEntry>,
    pub(crate) args: Vec<CString>,
    pub(crate) env: Vec<CString>,
}

impl WasiCtx {
    /// Make a new `WasiCtx` with some default settings.
    ///
    /// - File descriptors 0, 1, and 2 inherit stdin, stdout, and stderr from the host process.
    ///
    /// - Environment variables are inherited from the host process.
    ///
    /// To override these behaviors, use `WasiCtxBuilder`.
    pub fn new<S: AsRef<[u8]>>(args: impl Iterator<Item = S>) -> Result<Self> {
        WasiCtxBuilder::new()
            .and_then(|ctx| ctx.args(args))
            .and_then(|ctx| ctx.inherit_stdio())
            .and_then(|ctx| ctx.inherit_env())
            .and_then(|ctx| ctx.build())
    }

    /// Check if `WasiCtx` contains the specified raw WASI `fd`.
    pub(crate) unsafe fn contains_fd_entry(&self, fd: wasi::__wasi_fd_t) -> bool {
        self.fds.contains_key(&fd)
    }

    /// Get an immutable `FdEntry` corresponding to the specified raw WASI `fd`.
    pub(crate) unsafe fn get_fd_entry(&self, fd: wasi::__wasi_fd_t) -> Result<&FdEntry> {
        self.fds.get(&fd).ok_or(Error::EBADF)
    }

    /// Get a mutable `FdEntry` corresponding to the specified raw WASI `fd`.
    pub(crate) unsafe fn get_fd_entry_mut(
        &mut self,
        fd: wasi::__wasi_fd_t,
    ) -> Result<&mut FdEntry> {
        self.fds.get_mut(&fd).ok_or(Error::EBADF)
    }

    /// Insert the specified `FdEntry` into the `WasiCtx` object.
    ///
    /// The `FdEntry` will automatically get another free raw WASI `fd` assigned. Note that
    /// the two subsequent free raw WASI `fd`s do not have to be stored contiguously.
    pub(crate) fn insert_fd_entry(&mut self, fe: FdEntry) -> Result<wasi::__wasi_fd_t> {
        // never insert where stdio handles usually are
        let mut fd = 3;
        while self.fds.contains_key(&fd) {
            if let Some(next_fd) = fd.checked_add(1) {
                fd = next_fd;
            } else {
                return Err(Error::EMFILE);
            }
        }
        self.fds.insert(fd, fe);
        Ok(fd)
    }

    /// Insert the specified `FdEntry` with the specified raw WASI `fd` key into the `WasiCtx`
    /// object.
    pub(crate) fn insert_fd_entry_at(
        &mut self,
        fd: wasi::__wasi_fd_t,
        fe: FdEntry,
    ) -> Option<FdEntry> {
        self.fds.insert(fd, fe)
    }

    /// Remove `FdEntry` corresponding to the specified raw WASI `fd` from the `WasiCtx` object.
    pub(crate) fn remove_fd_entry(&mut self, fd: wasi::__wasi_fd_t) -> Result<FdEntry> {
        self.fds.remove(&fd).ok_or(Error::EBADF)
    }
}
