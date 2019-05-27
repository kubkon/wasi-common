#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use super::host_impl;
use crate::ctx::WasiCtx;
use crate::host;

use std::ffi::{OsStr, OsString};
use std::os::windows::prelude::{OsStrExt, OsStringExt, RawHandle};

/// Normalizes a path to ensure that the target path is located under the directory provided.
pub fn path_get<P: AsRef<OsStr>>(
    wasi_ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: P,
    needed_base: host::__wasi_rights_t,
    needed_inheriting: host::__wasi_rights_t,
    needs_final_component: bool,
) -> Result<(RawHandle, OsString), host::__wasi_errno_t> {
    Err(host::__WASI_EBADF)
}
