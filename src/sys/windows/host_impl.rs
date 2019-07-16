//! WASI host types specific to Windows host.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
use crate::host::{self, AsInner, FromInner, RawString};
use std::ffi::{OsStr, OsString};
use std::marker::PhantomData;
use std::os::windows::prelude::{OsStrExt, OsStringExt};
use std::slice;
use std::str;

pub fn errno_from_win(error: winx::winerror::WinError) -> host::__wasi_errno_t {
    // TODO: implement error mapping between Windows and WASI
    use winx::winerror::WinError::*;
    match error {
        ERROR_SUCCESS => host::__WASI_ESUCCESS,
        ERROR_BAD_ENVIRONMENT => host::__WASI_E2BIG,
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => host::__WASI_ENOENT,
        ERROR_TOO_MANY_OPEN_FILES => host::__WASI_ENFILE,
        ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION => host::__WASI_EACCES,
        ERROR_INVALID_HANDLE | ERROR_INVALID_NAME => host::__WASI_EBADF,
        ERROR_NOT_ENOUGH_MEMORY | ERROR_OUTOFMEMORY => host::__WASI_ENOMEM,
        ERROR_DIR_NOT_EMPTY => host::__WASI_ENOTEMPTY,
        ERROR_DEV_NOT_EXIST => host::__WASI_ENODEV,
        ERROR_NOT_READY | ERROR_BUSY => host::__WASI_EBUSY,
        ERROR_NOT_SUPPORTED => host::__WASI_ENOTSUP,
        ERROR_FILE_EXISTS => host::__WASI_EEXIST,
        ERROR_BROKEN_PIPE => host::__WASI_EPIPE,
        ERROR_BUFFER_OVERFLOW => host::__WASI_ENAMETOOLONG,
        ERROR_DISK_FULL => host::__WASI_ENOSPC,
        ERROR_SHARING_BUFFER_EXCEEDED => host::__WASI_ENFILE,
        _ => host::__WASI_ENOTSUP,
    }
}

pub fn win_from_fdflags(
    fdflags: host::__wasi_fdflags_t,
) -> (winx::file::AccessRight, winx::file::FlagsAndAttributes) {
    use winx::file::{AccessRight, FlagsAndAttributes};
    // TODO verify this!
    let mut win_rights = AccessRight::empty();
    let mut win_flags_attrs = FlagsAndAttributes::empty();

    if fdflags & host::__WASI_FDFLAG_NONBLOCK != 0 {
        win_flags_attrs.insert(FlagsAndAttributes::FILE_FLAG_OVERLAPPED);
    }
    if fdflags & host::__WASI_FDFLAG_APPEND != 0 {
        win_rights.insert(AccessRight::FILE_APPEND_DATA);
    }
    if fdflags & host::__WASI_FDFLAG_DSYNC != 0
        || fdflags & host::__WASI_FDFLAG_RSYNC != 0
        || fdflags & host::__WASI_FDFLAG_SYNC != 0
    {
        win_rights.insert(AccessRight::SYNCHRONIZE);
    }
    (win_rights, win_flags_attrs)
}

pub fn fdflags_from_win(rights: winx::file::AccessRight) -> host::__wasi_fdflags_t {
    use winx::file::AccessRight;
    let mut fdflags = 0;
    // TODO verify this!
    if rights.contains(AccessRight::FILE_APPEND_DATA) {
        fdflags |= host::__WASI_FDFLAG_APPEND;
    }
    if rights.contains(AccessRight::SYNCHRONIZE) {
        fdflags |= host::__WASI_FDFLAG_DSYNC;
        fdflags |= host::__WASI_FDFLAG_RSYNC;
        fdflags |= host::__WASI_FDFLAG_SYNC;
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

pub(crate) trait RawStringExt {
    fn from_bytes(slice: &[u8]) -> Result<RawString, host::__wasi_errno_t>;
    fn to_bytes(&self) -> Result<Vec<u8>, host::__wasi_errno_t>;
    fn contains(&self, c: &u8) -> Result<bool, host::__wasi_errno_t>;
    fn ends_with(&self, c: &[u8]) -> Result<bool, host::__wasi_errno_t>;
}

impl RawStringExt for RawString {
    fn from_bytes(slice: &[u8]) -> Result<RawString, host::__wasi_errno_t> {
        to_utf16(slice).map(|s| FromInner::from_inner(OsString::from_wide(&s)))
    }

    fn to_bytes(&self) -> Result<Vec<u8>, host::__wasi_errno_t> {
        self.as_inner()
            .to_str()
            .map(|s| s.as_bytes().to_owned())
            .ok_or(host::__WASI_EILSEQ)
    }

    fn contains(&self, c: &u8) -> Result<bool, host::__wasi_errno_t> {
        let c = &[*c];
        let mut u16s = to_utf16(c)?;
        if u16s.len() > 1 {
            return Err(host::__WASI_EILSEQ);
        }
        u16s.pop()
            .map(|c| self.as_inner().encode_wide().find(|&x| x == c).is_some())
            .ok_or(host::__WASI_EILSEQ)
    }

    fn ends_with(&self, cs: &[u8]) -> Result<bool, host::__wasi_errno_t> {
        let cs = to_utf16(cs)?;
        let ss: Vec<u16> = self.as_inner().encode_wide().collect();
        Ok(ss
            .into_iter()
            .rev()
            .zip(cs.into_iter().rev())
            .all(|(l, r)| l == r))
    }
}

fn to_utf16(slice: &[u8]) -> Result<Vec<u16>, host::__wasi_errno_t> {
    str::from_utf8(slice)
        .map(|s| s.encode_utf16().collect())
        .map_err(|_| host::__WASI_EILSEQ)
}
