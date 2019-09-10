use crate::WasiCtx;

lazy_static! {
    // TODO: Should we allow the context to be passed alternate arguments?
    pub(crate) static ref CONTEXT: WasiCtx =
        WasiCtx::new(std::env::args()).expect("initializing WASI state");
}
