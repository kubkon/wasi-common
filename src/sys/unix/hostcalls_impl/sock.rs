#![allow(non_camel_case_types)]
use crate::{host, wasm32, Result};
use crate::sys::host_impl;
use nix::sys::socket::{self, AddressFamily, SockType, SockFlag, SockProtocol};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::unix::prelude::{RawFd};

pub(crate) fn sock_connect(addr: impl ToSocketAddrs) -> Result<TcpStream> {
    TcpStream::connect(addr)
        .map_err(|_| 1) // some number just to compile
}
