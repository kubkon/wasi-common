use super::host;
use crate::sys::fdentry_impl;

use std::fs;
use std::io;
use std::mem::ManuallyDrop;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Descriptor {
    File(fs::File),
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Debug)]
pub struct FdObject {
    pub file_type: host::__wasi_filetype_t,
    pub descriptor: ManuallyDrop<Descriptor>,
    pub needs_close: bool,
    // TODO: directories
}

#[derive(Debug)]
pub struct FdEntry {
    pub fd_object: FdObject,
    pub rights_base: host::__wasi_rights_t,
    pub rights_inheriting: host::__wasi_rights_t,
    pub preopen_path: Option<PathBuf>,
}

impl Drop for FdObject {
    fn drop(&mut self) {
        if self.needs_close {
            unsafe { ManuallyDrop::drop(&mut self.descriptor) };
        }
    }
}

impl FdEntry {
    pub fn from(file: fs::File) -> Self {
        let (file_type, rights_base, rights_inheriting) =
            fdentry_impl::determine_type_and_access_rights(&file)
                .expect("could determine type and access rights");

        Self {
            fd_object: FdObject {
                file_type,
                descriptor: ManuallyDrop::new(Descriptor::File(file)),
                needs_close: true,
            },
            rights_base,
            rights_inheriting,
            preopen_path: None,
        }
    }

    pub fn duplicate(file: &fs::File) -> Self {
        let file = file.try_clone().expect("could duplicate file");
        Self::from(file)
    }

    pub fn duplicate_stdin() -> Self {
        let stdin = io::stdin();
        let (file_type, rights_base, rights_inheriting) =
            fdentry_impl::determine_type_and_access_rights(&stdin)
                .expect("could determinte type and access rights for STDIN");

        Self {
            fd_object: FdObject {
                file_type,
                descriptor: ManuallyDrop::new(Descriptor::Stdin),
                needs_close: true,
            },
            rights_base,
            rights_inheriting,
            preopen_path: None,
        }
    }

    pub fn duplicate_stdout() -> Self {
        let stdout = io::stdout();
        let (file_type, rights_base, rights_inheriting) =
            fdentry_impl::determine_type_and_access_rights(&stdout)
                .expect("could determinte type and access rights for STDOUT");

        Self {
            fd_object: FdObject {
                file_type,
                descriptor: ManuallyDrop::new(Descriptor::Stdout),
                needs_close: true,
            },
            rights_base,
            rights_inheriting,
            preopen_path: None,
        }
    }

    pub fn duplicate_stderr() -> Self {
        let stderr = io::stderr();
        let (file_type, rights_base, rights_inheriting) =
            fdentry_impl::determine_type_and_access_rights(&stderr)
                .expect("could determinte type and access rights for STDERR");

        Self {
            fd_object: FdObject {
                file_type,
                descriptor: ManuallyDrop::new(Descriptor::Stderr),
                needs_close: true,
            },
            rights_base,
            rights_inheriting,
            preopen_path: None,
        }
    }
}
