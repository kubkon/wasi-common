#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use crate::ctx::WasiCtx;
use crate::wasm32;
use wasi_common_cbindgen::wasi_common_cbindgen;
use std::net::ToSocketAddrs;
use std::net::TcpStream;

#[wasi_common_cbindgen]
pub unsafe fn sock_recv(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasm32::__wasi_fd_t,
    ri_data: wasm32::uintptr_t,
    ri_data_len: wasm32::size_t,
    ri_flags: wasm32::__wasi_riflags_t,
    ro_datalen: wasm32::uintptr_t,
    ro_flags: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("sock_recv")
}

#[wasi_common_cbindgen]
pub unsafe fn sock_send(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasm32::__wasi_fd_t,
    si_data: wasm32::uintptr_t,
    si_data_len: wasm32::size_t,
    si_flags: wasm32::__wasi_siflags_t,
    so_datalen: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("sock_send")
}

#[wasi_common_cbindgen]
pub unsafe fn sock_shutdown(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasm32::__wasi_fd_t,
    how: wasm32::__wasi_sdflags_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("sock_shutdown")
}

hostcalls! {
    pub fn sock_connect(
        wasi_ctx: &WasiCtx,
        memory: &mut [u8],
        sock: wasm32::__wasi_fd_t,
        addr_ptr: wasm32::uintptr_t,
        addr_len: wasm32::size_t,
    ) -> wasm32::__wasi_errno_t;

    pub fn sock_socket(
        wasi_ctx: &mut WasiCtx,
        memory: &mut [u8],
        sock_domain: wasm32::int32_t,
        // socket type
        // DGRAM 5
        // STREAM 6
        sock_type: wasm32::__wasi_filetype_t,
        sock_protocol: wasm32::int32_t,
        fd_out_ptr: wasm32::uintptr_t,
    ) -> wasm32::__wasi_errno_t;
}
