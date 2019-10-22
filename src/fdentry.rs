use crate::sys::fdentry_impl::{determine_type_and_access_rights, OsFile};
use crate::{host, Error, Result};
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::{fs, io, net};

#[derive(Debug)]
pub struct SocketDetails {
    pub socket_domain: i32,
    pub socket_type: u8,
    pub socket_protocol: i32,
}

#[derive(Debug)]
pub(crate) enum Descriptor {
    OsFile(OsFile),
    Socket(net::TcpStream),
    SocketFd(SocketDetails), // stores allocated fd, and options: domain, type and protocol
    Stdin,
    Stdout,
    Stderr,
}

impl Descriptor {
    pub(crate) fn as_file(&self) -> Result<&OsFile> {
        match self {
            Self::OsFile(file) => Ok(file),
            _ => Err(Error::EBADF),
        }
    }

    pub(crate) fn as_file_mut(&mut self) -> Result<&mut OsFile> {
        match self {
            Self::OsFile(file) => Ok(file),
            _ => Err(Error::EBADF),
        }
    }

    pub(crate) fn is_file(&self) -> bool {
        match self {
            Self::OsFile(_) => true,
            _ => false,
        }
    }

    #[allow(unused)]
    pub(crate) fn as_socket(&self) -> Result<&net::TcpStream> {
        match self {
            Descriptor::Socket(s) => Ok(s),
            // TODO: add a separate error code?
            _ => Err(Error::EBADF),
        }
    }

    pub(crate) fn is_socket(&self) -> bool {
        match self {
            Descriptor::Socket(_) => true,
            _ => false,
        }
    }

    pub(crate) fn is_socket_fd(&self) -> bool {
        match self {
            Descriptor::SocketFd(_) => true,
            _ => false,
        }
    }

    pub(crate) fn is_stdin(&self) -> bool {
        match self {
            Self::Stdin => true,
            _ => false,
        }
    }

    #[allow(unused)]
    pub(crate) fn is_stdout(&self) -> bool {
        match self {
            Self::Stdout => true,
            _ => false,
        }
    }

    #[allow(unused)]
    pub(crate) fn is_stderr(&self) -> bool {
        match self {
            Self::Stderr => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub(crate) struct FdObject {
    pub(crate) file_type: host::__wasi_filetype_t,
    pub(crate) descriptor: ManuallyDrop<Descriptor>,
    pub(crate) needs_close: bool,
    // TODO: directories
}

#[derive(Debug)]
pub(crate) struct FdEntry {
    pub(crate) fd_object: FdObject,
    pub(crate) rights_base: host::__wasi_rights_t,
    pub(crate) rights_inheriting: host::__wasi_rights_t,
    pub(crate) preopen_path: Option<PathBuf>,
}

impl Drop for FdObject {
    fn drop(&mut self) {
        if self.needs_close {
            unsafe { ManuallyDrop::drop(&mut self.descriptor) };
        }
    }
}

impl FdEntry {
    pub(crate) fn from(file: fs::File) -> Result<Self> {
        unsafe { determine_type_and_access_rights(&file) }.map(
            |(file_type, rights_base, rights_inheriting)| Self {
                fd_object: FdObject {
                    file_type,
                    descriptor: ManuallyDrop::new(Descriptor::OsFile(OsFile::from(file))),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub(crate) fn from_socket_details(details: SocketDetails) -> Result<Self> {
        fdentry_impl::determine_type_and_access_rights_for_socket().map(
            |(file_type, rights_base, rights_inheriting)| Self {
                fd_object: FdObject {
                    file_type,
                    descriptor: ManuallyDrop::new(Descriptor::SocketFd(details)),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub(crate) fn duplicate(file: &fs::File) -> Result<Self> {
        Self::from(file.try_clone()?)
    }

    pub(crate) fn duplicate_stdin() -> Result<Self> {
        unsafe { determine_type_and_access_rights(&io::stdin()) }.map(
            |(file_type, rights_base, rights_inheriting)| Self {
                fd_object: FdObject {
                    file_type,
                    descriptor: ManuallyDrop::new(Descriptor::Stdin),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub(crate) fn duplicate_stdout() -> Result<Self> {
        unsafe { determine_type_and_access_rights(&io::stdout()) }.map(
            |(file_type, rights_base, rights_inheriting)| Self {
                fd_object: FdObject {
                    file_type,
                    descriptor: ManuallyDrop::new(Descriptor::Stdout),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub(crate) fn duplicate_stderr() -> Result<Self> {
        unsafe { determine_type_and_access_rights(&io::stderr()) }.map(
            |(file_type, rights_base, rights_inheriting)| Self {
                fd_object: FdObject {
                    file_type,
                    descriptor: ManuallyDrop::new(Descriptor::Stderr),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }
}
