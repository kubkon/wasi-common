#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::sys::errno_from_ioerror;
use crate::sys::host_impl;
use crate::{host, Result};
use std::fs::File;
use std::os::windows::prelude::AsRawHandle;
use std::path::{Path, PathBuf};

pub(crate) fn path_open_rights(
    rights_base: host::__wasi_rights_t,
    rights_inheriting: host::__wasi_rights_t,
    oflags: host::__wasi_oflags_t,
    fs_flags: host::__wasi_fdflags_t,
) -> (host::__wasi_rights_t, host::__wasi_rights_t) {
    use winx::file::{AccessRight, CreationDisposition};

    // which rights are needed on the dirfd?
    let mut needed_base = host::__WASI_RIGHT_PATH_OPEN;
    let mut needed_inheriting = rights_base | rights_inheriting;

    // convert open flags
    let (win_create_disp, _) = host_impl::win_from_oflags(oflags);
    if win_create_disp == CreationDisposition::CREATE_NEW {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    } else if win_create_disp == CreationDisposition::CREATE_ALWAYS {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    } else if win_create_disp == CreationDisposition::TRUNCATE_EXISTING {
        needed_base |= host::__WASI_RIGHT_PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    let win_fdflags_res = host_impl::win_from_fdflags(fs_flags);
    if win_fdflags_res.0.contains(AccessRight::SYNCHRONIZE) {
        needed_inheriting |= host::__WASI_RIGHT_FD_DATASYNC;
        needed_inheriting |= host::__WASI_RIGHT_FD_SYNC;
    }

    (needed_base, needed_inheriting)
}

pub(crate) fn openat(dirfd: &File, path: &str) -> Result<File> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;

    let path = concatenate_if_relative(dirfd, Path::new(path))?;
    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map_err(errno_from_ioerror)
}

pub(crate) fn readlinkat(dirfd: &File, path: &str) -> Result<String> {
    use std::fs;

    let path = concatenate_if_relative(dirfd, Path::new(path))?;
    fs::read_link(path)
        .map_err(errno_from_ioerror)
        .and_then(|path| path.to_str().map(String::from).ok_or(host::__WASI_EILSEQ))
}

pub(crate) fn concatenate_if_relative<P: AsRef<Path>>(dirfd: &File, path: P) -> Result<PathBuf> {
    use winx::file::get_path_by_handle;

    // check if specified path is absolute
    let out_path = if path.as_ref().is_absolute() {
        path.as_ref().to_owned()
    } else {
        let dir_path =
            get_path_by_handle(dirfd.as_raw_handle()).map_err(host_impl::errno_from_win)?;
        // concatenate paths
        let mut out_path = PathBuf::from(&dir_path);
        out_path.push(path.as_ref());
        out_path.into()
    };

    Ok(out_path)
}
