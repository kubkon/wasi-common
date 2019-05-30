//! WASI host types specific to Windows host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
use crate::host;

use std::slice;

pub fn errno_from_win(error: winx::winerror::WinError) -> host::__wasi_errno_t {
    // TODO: implement error mapping between Windows and WASI
    match error {
        _ => host::__WASI_EBADF,
    }
}

pub unsafe fn ciovec_to_win<'a>(ciovec: &'a host::__wasi_ciovec_t) -> winx::io::IoVec<'a> {
    let slice = slice::from_raw_parts(ciovec.buf as *const u8, ciovec.buf_len);
    winx::io::IoVec::new(slice)
}

pub unsafe fn ciovec_to_win_mut<'a>(
    ciovec: &'a mut host::__wasi_ciovec_t,
) -> winx::io::IoVecMut<'a> {
    let slice = slice::from_raw_parts_mut(ciovec.buf as *mut u8, ciovec.buf_len);
    winx::io::IoVecMut::new(slice)
}

pub unsafe fn iovec_to_win<'a>(iovec: &'a host::__wasi_iovec_t) -> winx::io::IoVec<'a> {
    let slice = slice::from_raw_parts(iovec.buf as *const u8, iovec.buf_len);
    winx::io::IoVec::new(slice)
}

pub unsafe fn iovec_to_win_mut<'a>(iovec: &'a mut host::__wasi_iovec_t) -> winx::io::IoVecMut<'a> {
    let slice = slice::from_raw_parts_mut(iovec.buf as *mut u8, iovec.buf_len);
    winx::io::IoVecMut::new(slice)
}

pub fn fdflags_from_win(rights: winx::file::AccessRight) -> host::__wasi_fdflags_t {
    use winx::file::AccessRight;
    let mut fdflags = 0;
    // TODO verify this!
    if rights.contains(AccessRight::FILE_APPEND_DATA) {
        fdflags |= host::__WASI_FDFLAG_APPEND;
    }
    if rights.contains(AccessRight::SYNCHRONIZE) {
        if rights.contains(AccessRight::FILE_WRITE_DATA) {
            fdflags |= host::__WASI_FDFLAG_DSYNC;
        }
        if rights.contains(AccessRight::FILE_READ_DATA) {
            fdflags |= host::__WASI_FDFLAG_RSYNC;
        }
        if rights.contains(AccessRight::FILE_WRITE_ATTRIBUTES) {
            fdflags |= host::__WASI_FDFLAG_SYNC;
        }
    }
    // The NONBLOCK equivalent is FILE_FLAG_OVERLAPPED
    // but it seems winapi doesn't provide a mechanism
    // for checking whether the handle supports async IO.
    // On the contrary, I've found some dicsussion online
    // which suggests that on Windows all handles should
    // generally be assumed to be opened with async support
    // and then the program should fallback should that **not**
    // be the case at the time of the operation.
    // TODO: this requires further investigation
    fdflags
}

pub fn win_from_oflags(
    oflags: host::__wasi_oflags_t,
) -> (
    winx::file::CreationDisposition,
    winx::file::FlagsAndAttributes,
) {
    use winx::file::{CreationDisposition, FlagsAndAttributes};

    let win_flags_attrs = if oflags & host::__WASI_O_DIRECTORY != 0 {
        FlagsAndAttributes::FILE_FLAG_BACKUP_SEMANTICS
    } else {
        FlagsAndAttributes::FILE_ATTRIBUTE_NORMAL
    };

    let win_disp = if oflags & host::__WASI_O_CREAT != 0 && oflags & host::__WASI_O_EXCL != 0 {
        CreationDisposition::CREATE_NEW
    } else if oflags & host::__WASI_O_CREAT != 0 {
        CreationDisposition::CREATE_ALWAYS
    } else if oflags & host::__WASI_O_TRUNC != 0 {
        CreationDisposition::TRUNCATE_EXISTING
    } else {
        CreationDisposition::OPEN_EXISTING
    };

    (win_disp, win_flags_attrs)
}
