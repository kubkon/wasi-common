use crate::WasiCtx;
use std::sync::Mutex;

lazy_static! {
    // TODO: Should we allow the context to be passed alternate arguments?
    pub(crate) static ref CONTEXT: Mutex<WasiCtx> =
        Mutex::new(WasiCtx::new(std::env::args()).expect("initializing WASI state"));
}
