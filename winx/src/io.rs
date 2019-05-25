use crate::{winerror, Result};

use std::marker::PhantomData;
use std::os::windows::prelude::*;
use std::slice;
use winapi::shared::{ntdef, ws2def};

// these will be obsolete once https://github.com/rust-lang/rust/pull/60334
// lands in stable
pub struct IoVec<'a> {
    vec: ws2def::WSABUF,
    _p: PhantomData<&'a [u8]>,
}

pub struct IoVecMut<'a> {
    vec: ws2def::WSABUF,
    _p: PhantomData<&'a mut [u8]>,
}

impl<'a> IoVec<'a> {
    #[inline]
    pub fn new(buf: &'a [u8]) -> Self {
        assert!(buf.len() <= ntdef::ULONG::max_value() as usize);
        Self {
            vec: ws2def::WSABUF {
                len: buf.len() as ntdef::ULONG,
                buf: buf.as_ptr() as *mut u8 as *mut ntdef::CHAR,
            },
            _p: PhantomData,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.vec.buf as *mut u8, self.vec.len as usize) }
    }
}

impl<'a> IoVecMut<'a> {
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> Self {
        assert!(buf.len() <= ntdef::ULONG::max_value() as usize);
        Self {
            vec: ws2def::WSABUF {
                len: buf.len() as ntdef::ULONG,
                buf: buf.as_mut_ptr() as *mut u8 as *mut ntdef::CHAR,
            },
            _p: PhantomData,
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.vec.buf as *mut u8, self.vec.len as usize) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.vec.buf as *mut u8, self.vec.len as usize) }
    }
}

pub fn writev<'a>(raw_handle: RawHandle, iovecs: &[IoVec<'a>]) -> Result<usize> {
    use winapi::shared::minwindef::{DWORD, FALSE, LPVOID};
    use winapi::um::fileapi::WriteFile;

    let buf = iovecs
        .iter()
        .find(|b| !b.as_slice().is_empty())
        .map_or(&[][..], |b| b.as_slice());

    let mut host_nwritten = 0;
    let len = std::cmp::min(buf.len(), <DWORD>::max_value() as usize) as DWORD;
    unsafe {
        if WriteFile(
            raw_handle,
            buf.as_ptr() as LPVOID,
            len,
            &mut host_nwritten,
            std::ptr::null_mut(),
        ) == FALSE
        {
            return Err(winerror::WinError::last());
        }
    };

    Ok(host_nwritten as usize)
}
