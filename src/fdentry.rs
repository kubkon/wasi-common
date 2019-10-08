use crate::sys::fdentry_impl::{determine_type_and_access_rights, OsFile};
use crate::{host, Error, Result};
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug)]
pub struct FileDetails {
    pub(crate) file: OsFile,
    pub(crate) preopen_path: Option<PathBuf>,
}

#[derive(Debug)]
pub(crate) enum Descriptor {
    OsFile(FileDetails),
    Stdin,
    Stdout,
    Stderr,
}

impl Descriptor {
    pub(crate) fn as_file(&self) -> Result<&OsFile> {
        match self {
            Self::OsFile(details) => Ok(&details.file),
            _ => Err(Error::EBADF),
        }
    }

    pub(crate) fn as_file_mut(&mut self) -> Result<&mut OsFile> {
        match self {
            Self::OsFile(details) => Ok(&mut details.file),
            _ => Err(Error::EBADF),
        }
    }

    pub fn as_file_details(&self) -> Result<&FileDetails> {
        match self {
            Descriptor::OsFile(details) => Ok(details),
            _ => Err(Error::EBADF),
        }
    }

    pub fn as_file_details_mut(&mut self) -> Result<&mut FileDetails> {
        match self {
            Descriptor::OsFile(ref mut details) => Ok(details),
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
                    descriptor: ManuallyDrop::new(Descriptor::OsFile(FileDetails {
                        file: OsFile::from(file),
                        preopen_path: None,
                    })),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
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
            },
        )
    }
}
