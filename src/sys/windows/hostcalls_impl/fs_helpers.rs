#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::sys::host_impl;
use crate::{host, Result};
use std::fs::File;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle};

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
    use winx::file::{openat, AccessRight, CreationDisposition, FlagsAndAttributes};

    openat(
        dirfd.as_raw_handle(),
        path,
        AccessRight::FILE_GENERIC_READ,
        CreationDisposition::OPEN_EXISTING,
        FlagsAndAttributes::FILE_FLAG_BACKUP_SEMANTICS,
    )
    .map(|new_handle| unsafe { File::from_raw_handle(new_handle) })
    .map_err(host_impl::errno_from_win)
}

pub(crate) fn readlinkat(_dirfd: &File, _path: &str) -> Result<String> {
    unimplemented!("readlinkat")
}
