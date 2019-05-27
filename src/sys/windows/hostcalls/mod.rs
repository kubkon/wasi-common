//! Windows-specific hostcalls that implement
//! [WASI](https://github.com/CraneStation/wasmtime-wasi/blob/wasi/docs/WASI-overview.md).
mod fs;
mod fs_helpers;
mod misc;
mod sock;

use super::fdentry;
use super::host_impl;

pub use self::fs::*;
pub use self::misc::*;
pub use self::sock::*;
