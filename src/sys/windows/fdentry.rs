use crate::host;

use std::fs::File;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FdObject {
    pub ty: host::__wasi_filetype_t,
    pub raw_handle: RawHandle,
    pub needs_close: bool,
    // TODO: directories
}

#[derive(Clone, Debug)]
pub struct FdEntry {
    pub fd_object: FdObject,
    pub rights_base: host::__wasi_rights_t,
    pub rights_inheriting: host::__wasi_rights_t,
    pub preopen_path: Option<PathBuf>,
}

impl Drop for FdObject {
    fn drop(&mut self) {
        if self.needs_close {
            winx::handle::close(self.raw_handle)
                .unwrap_or_else(|e| eprintln!("FdObject::drop(): {}", e))
        }
    }
}

impl FdEntry {
    pub fn from_file(file: File) -> Self {
        unsafe { Self::from_raw_handle(file.into_raw_handle()) }
    }

    pub fn duplicate<F: AsRawHandle>(fd: &F) -> Self {
        unsafe { Self::from_raw_handle(winx::handle::dup(fd.as_raw_handle()).unwrap()) }
    }
}

impl FromRawHandle for FdEntry {
    // TODO: implement
    unsafe fn from_raw_handle(raw_handle: RawHandle) -> Self {
        let (ty, rights_base, rights_inheriting) =
            determine_type_rights(raw_handle).expect("can determine file rights");

        Self {
            fd_object: FdObject {
                ty,
                raw_handle,
                needs_close: true,
            },
            rights_base,
            rights_inheriting,
            preopen_path: None,
        }
    }
}

pub unsafe fn determine_type_rights(
    raw_handle: RawHandle,
) -> Result<
    (
        host::__wasi_filetype_t,
        host::__wasi_rights_t,
        host::__wasi_rights_t,
    ),
    host::__wasi_errno_t,
> {
    use winapi::um::fileapi::GetFileType;
    use winapi::um::winbase;

    let (ty, rights_base, rights_inheriting) = {
        match GetFileType(raw_handle) {
            winbase::FILE_TYPE_CHAR => {
                // character file: LPT device or console
                // TODO: rule out LPT device
                (
                    host::__WASI_FILETYPE_CHARACTER_DEVICE,
                    host::RIGHTS_TTY_BASE,
                    host::RIGHTS_TTY_BASE,
                )
            }
            winbase::FILE_TYPE_DISK => {
                // disk file: file, dir or disk device
                let file = File::from_raw_handle(raw_handle);
                let meta = file.metadata();
                std::mem::forget(file);
                let meta = meta.map_err(|_| host::__WASI_EINVAL)?;
                if meta.is_dir() {
                    (
                        host::__WASI_FILETYPE_DIRECTORY,
                        host::RIGHTS_DIRECTORY_BASE,
                        host::RIGHTS_DIRECTORY_INHERITING,
                    )
                } else if meta.is_file() {
                    let mut rights = host::RIGHTS_REGULAR_FILE_BASE;
                    if meta.permissions().readonly() {
                        // on Windows, there is only a readonly flag available
                        rights &= !host::__WASI_RIGHT_FD_WRITE;
                    }

                    (
                        host::__WASI_FILETYPE_REGULAR_FILE,
                        rights,
                        host::RIGHTS_REGULAR_FILE_INHERITING,
                    )
                } else {
                    return Err(host::__WASI_EINVAL);
                }
            }
            winbase::FILE_TYPE_PIPE => {
                // pipe object: socket, named pipe or anonymous pipe
                unimplemented!()
            }
            _ => return Err(host::__WASI_EINVAL),
        }
    };
    Ok((ty, rights_base, rights_inheriting))
}
