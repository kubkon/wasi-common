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
