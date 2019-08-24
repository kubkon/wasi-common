macro_rules! hostcalls {
    ($(
            $(#[doc=$doc:literal])*
            pub fn $name:ident($($arg:ident: $ty:ty,)*) -> $ret:ty;
    )*) => (
        $(
            $(#[doc=$doc])*
            #[wasi_common_cbindgen::wasi_common_cbindgen]
            pub fn $name($($arg: $ty,)*) -> $ret {
                let ret = match crate::hostcalls_impl::$name($($arg,)*) {
                    Ok(()) => crate::host::__WASI_ESUCCESS,
                    Err(e) => e,
                };

                crate::hostcalls::return_enc_errno(ret)
            }
        )*
    )
}
