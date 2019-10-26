//! WASI host types. These are types that contain raw pointers and `usize`
//! values, and so are platform-specific.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::wasi::*;
use std::{io, slice};
use wig::witx_host_types;

witx_host_types!("unstable" "wasi_unstable_preview0");

#[allow(unused)]
pub(crate) unsafe fn ciovec_to_host(ciovec: &__wasi_ciovec_t) -> io::IoSlice {
    let slice = slice::from_raw_parts(ciovec.buf as *const u8, ciovec.buf_len);
    io::IoSlice::new(slice)
}

#[allow(unused)]
pub(crate) unsafe fn ciovec_to_host_mut(ciovec: &mut __wasi_ciovec_t) -> io::IoSliceMut {
    let slice = slice::from_raw_parts_mut(ciovec.buf as *mut u8, ciovec.buf_len);
    io::IoSliceMut::new(slice)
}

pub(crate) unsafe fn iovec_to_host(iovec: &__wasi_iovec_t) -> io::IoSlice {
    let slice = slice::from_raw_parts(iovec.buf as *const u8, iovec.buf_len);
    io::IoSlice::new(slice)
}

pub(crate) unsafe fn iovec_to_host_mut(iovec: &mut __wasi_iovec_t) -> io::IoSliceMut {
    let slice = slice::from_raw_parts_mut(iovec.buf as *mut u8, iovec.buf_len);
    io::IoSliceMut::new(slice)
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn bindgen_test_layout___wasi_prestat_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_prestat_t>(),
            16usize,
            concat!("Size of: ", stringify!(__wasi_prestat_t))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_prestat_t>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_prestat_t))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_prestat_t>())).pr_type as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_t),
                "::",
                stringify!(pr_type)
            )
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_prestat_t>())).u as *const _ as usize },
            8usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_t),
                "::",
                stringify!(u)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_prestat_dir>(),
            8usize,
            concat!("Size of: ", stringify!(__wasi_prestat_dir))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_prestat_dir>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_prestat_dir))
        );
        assert_eq!(
            unsafe {
                &(*(::std::ptr::null::<__wasi_prestat_dir>())).pr_name_len as *const _ as usize
            },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_dir),
                "::",
                stringify!(pr_name_len)
            )
        );
    }

    #[test]
    fn bindgen_test_layout___wasi_prestat_t___wasi_prestat_u() {
        assert_eq!(
            ::std::mem::size_of::<__wasi_prestat_u>(),
            8usize,
            concat!("Size of: ", stringify!(__wasi_prestat_u))
        );
        assert_eq!(
            ::std::mem::align_of::<__wasi_prestat_u>(),
            8usize,
            concat!("Alignment of ", stringify!(__wasi_prestat_u))
        );
        assert_eq!(
            unsafe { &(*(::std::ptr::null::<__wasi_prestat_u>())).dir as *const _ as usize },
            0usize,
            concat!(
                "Offset of field: ",
                stringify!(__wasi_prestat_u),
                "::",
                stringify!(dir)
            )
        );
    }
}
