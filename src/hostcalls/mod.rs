mod fs;
mod misc;
mod sock;

pub use self::fs::*;
pub use self::misc::*;
pub use self::sock::*;

use crate::{memory, wasi};

fn return_enc_errno(errno: wasi::__wasi_errno_t) -> wasi::__wasi_errno_t {
    let errno = memory::enc_errno(errno);
    log::trace!("    -> errno={}", wasi::strerror(errno));
    errno
}
