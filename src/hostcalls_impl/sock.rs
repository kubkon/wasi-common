#![allow(non_camel_case_types)]
use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::{host, wasm32, Result};
use crate::sys::{hostcalls_impl};
use crate::fdentry::{SocketDetails, FdEntry};
use log::trace;
use std::net::ToSocketAddrs;

pub(crate) fn sock_connect(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    sock: wasm32::__wasi_fd_t,
    addr_ptr: wasm32::uintptr_t,
    addr_len: wasm32::size_t,
) -> Result<()> {
    let addr = dec_slice_of::<u8>(memory, addr_ptr, addr_len).and_then(host::path_from_slice)?;
    let fd = hostcalls_impl::sock_connect(addr)?;
    Ok(())
}

pub(crate) fn sock_socket(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    sock_domain: wasm32::int32_t,
    sock_type: wasm32::__wasi_filetype_t,
    sock_protocol: wasm32::int32_t,
    fd_out_ptr: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "sock_socket(sock_domain={:?}, sock_type={:?}, sock_protocol={:?}, fd_out_ptr={:#x?})",
        sock_domain,
        sock_type,
        sock_protocol,
        fd_out_ptr,
    );

    // pre-encode fd_out_ptr to -1 in case of error in creating a socket
    enc_fd_byref(memory, fd_out_ptr, wasm32::__wasi_fd_t::max_value())?;


    let details = SocketDetails {
        socket_domain: sock_domain,
        socket_type: sock_type,
        socket_protocol: sock_protocol,
    };
    let fe = FdEntry::from_socket_details(details)?;
    let sock_fd = wasi_ctx.insert_fd_entry(fe)?;

    enc_fd_byref(memory, fd_out_ptr, sock_fd)
}
