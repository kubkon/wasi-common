//! Windows-specific hostcalls that implement
//! [WASI](https://github.com/CraneStation/wasmtime-wasi/blob/wasi/docs/WASI-overview.md).
mod fs;
pub(crate) mod fs_helpers;
mod misc;
mod symlink;

pub(crate) use self::fs::*;
pub(crate) use self::misc::*;

use symlink::Symlink;
use std::path::PathBuf;
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref SYMLINKS: Mutex<HashMap<PathBuf, Symlink>> = Mutex::new(HashMap::new());
}