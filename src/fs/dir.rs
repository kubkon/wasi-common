use crate::fs::{ctx, error::wasi_errno_to_io_error, File};
use crate::{host, hostcalls};
use std::os::unix::ffi::OsStrExt;
use std::{io, path::Path};

/// A reference to an open directory on the filesystem.
pub struct Dir {
    fd: host::__wasi_fd_t,
}

impl Dir {
    /// Constructs a new instance of Self from the given raw WASI file descriptor.
    pub unsafe fn from_raw_wasi_fd(fd: host::__wasi_fd_t) -> Self {
        Self { fd }
    }

    /// Attempts to open a file in read-only mode.
    fn open_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<File> {
        let path = path.as_ref();
        let mut fd = 0;
        wasi_errno_to_io_error(hostcalls::path_open(
            &mut ctx::CONTEXT,
            self.fd,
            host::__WASI_LOOKUP_SYMLINK_FOLLOW,
            path.as_os_str().as_bytes(),
            path.as_os_str().len(),
            0,
            !0,
            !0,
            0,
            &mut fd,
        ))?;

        Ok(File::from_raw_wasi_fd(fd))
    }

    /// Attempts to open a directory.
    fn open_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<Self> {
        let path = path.as_ref();
        let mut fd = 0;
        wasi_errno_to_io_error(hostcalls::path_open(
            &mut ctx::CONTEXT,
            self.fd,
            host::__WASI_LOOKUP_SYMLINK_FOLLOW,
            path.as_os_str().as_bytes(),
            host::__WASI_O_DIRECTORY,
            !0,
            !0,
            0,
            &mut fd,
        ))?;

        Ok(Self::from_raw_wasi_fd(fd))
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        // Note that errors are ignored when closing a file descriptor. The
        // reason for this is that if an error occurs we don't actually know if
        // the file descriptor was closed or not, and if we retried (for
        // something like EINTR), we might close another valid file descriptor
        // opened after we closed ours.
        let _ = hostcalls::fd_close(&mut ctx::CONTEXT, self.fd);
    }
}
