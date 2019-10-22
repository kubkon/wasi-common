#![allow(non_camel_case_types)]
use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::sys::hostcalls_impl;
use crate::{host, wasm32, Error, Result};
use log::trace;
use std::convert::TryFrom;
use std::time::SystemTime;

pub(crate) fn args_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argv_ptr: wasm32::uintptr_t,
    argv_buf: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "args_get(argv_ptr={:#x?}, argv_buf={:#x?})",
        argv_ptr,
        argv_buf,
    );

    let mut argv_buf_offset = 0;
    let mut argv = vec![];

    for arg in wasi_ctx.args.iter() {
        let arg_bytes = arg.as_bytes_with_nul();
        let arg_ptr = argv_buf + argv_buf_offset;

        enc_slice_of(memory, arg_bytes, arg_ptr)?;

        argv.push(arg_ptr);

        let len = wasm32::uintptr_t::try_from(arg_bytes.len())?;
        argv_buf_offset = argv_buf_offset.checked_add(len).ok_or(Error::EOVERFLOW)?;
    }

    enc_slice_of(memory, argv.as_slice(), argv_ptr)
}

pub(crate) fn args_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    argc_ptr: wasm32::uintptr_t,
    argv_buf_size_ptr: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "args_sizes_get(argc_ptr={:#x?}, argv_buf_size_ptr={:#x?})",
        argc_ptr,
        argv_buf_size_ptr,
    );

    let argc = wasi_ctx.args.len();
    let argv_size = wasi_ctx
        .args
        .iter()
        .map(|arg| arg.as_bytes_with_nul().len())
        .sum();

    trace!("     | *argc_ptr={:?}", argc);

    enc_usize_byref(memory, argc_ptr, argc)?;

    trace!("     | *argv_buf_size_ptr={:?}", argv_size);

    enc_usize_byref(memory, argv_buf_size_ptr, argv_size)
}

pub(crate) fn environ_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_ptr: wasm32::uintptr_t,
    environ_buf: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "environ_get(environ_ptr={:#x?}, environ_buf={:#x?})",
        environ_ptr,
        environ_buf,
    );

    let mut environ_buf_offset = 0;
    let mut environ = vec![];

    for pair in wasi_ctx.env.iter() {
        let env_bytes = pair.as_bytes_with_nul();
        let env_ptr = environ_buf + environ_buf_offset;

        enc_slice_of(memory, env_bytes, env_ptr)?;

        environ.push(env_ptr);

        let len = wasm32::uintptr_t::try_from(env_bytes.len())?;
        environ_buf_offset = environ_buf_offset
            .checked_add(len)
            .ok_or(Error::EOVERFLOW)?;
    }

    enc_slice_of(memory, environ.as_slice(), environ_ptr)
}

pub(crate) fn environ_sizes_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    environ_count_ptr: wasm32::uintptr_t,
    environ_size_ptr: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "environ_sizes_get(environ_count_ptr={:#x?}, environ_size_ptr={:#x?})",
        environ_count_ptr,
        environ_size_ptr,
    );

    let environ_count = wasi_ctx.env.len();
    let environ_size = wasi_ctx
        .env
        .iter()
        .try_fold(0, |acc: u32, pair| {
            acc.checked_add(pair.as_bytes_with_nul().len() as u32)
        })
        .ok_or(Error::EOVERFLOW)?;

    trace!("     | *environ_count_ptr={:?}", environ_count);

    enc_usize_byref(memory, environ_count_ptr, environ_count)?;

    trace!("     | *environ_size_ptr={:?}", environ_size);

    enc_usize_byref(memory, environ_size_ptr, environ_size as usize)
}

pub(crate) fn random_get(
    memory: &mut [u8],
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
) -> Result<()> {
    use rand::{thread_rng, RngCore};

    trace!("random_get(buf_ptr={:#x?}, buf_len={:?})", buf_ptr, buf_len);

    let buf = dec_slice_of_mut::<u8>(memory, buf_ptr, buf_len)?;

    thread_rng().fill_bytes(buf);

    Ok(())
}

pub(crate) fn clock_res_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    resolution_ptr: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "clock_res_get(clock_id={:?}, resolution_ptr={:#x?})",
        clock_id,
        resolution_ptr,
    );

    let clock_id = dec_clockid(clock_id);
    let resolution = hostcalls_impl::clock_res_get(clock_id)?;

    trace!("     | *resolution_ptr={:?}", resolution);

    enc_timestamp_byref(memory, resolution_ptr, resolution)
}

pub(crate) fn clock_time_get(
    memory: &mut [u8],
    clock_id: wasm32::__wasi_clockid_t,
    precision: wasm32::__wasi_timestamp_t,
    time_ptr: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "clock_time_get(clock_id={:?}, precision={:?}, time_ptr={:#x?})",
        clock_id,
        precision,
        time_ptr,
    );

    let clock_id = dec_clockid(clock_id);
    let time = hostcalls_impl::clock_time_get(clock_id)?;

    trace!("     | *time_ptr={:?}", time);

    enc_timestamp_byref(memory, time_ptr, time)
}

pub(crate) fn poll_oneoff(
    memory: &mut [u8],
    input: wasm32::uintptr_t,
    output: wasm32::uintptr_t,
    nsubscriptions: wasm32::size_t,
    nevents: wasm32::uintptr_t,
) -> Result<()> {
    trace!(
        "poll_oneoff(input={:#x?}, output={:#x?}, nsubscriptions={}, nevents={:#x?})",
        input,
        output,
        nsubscriptions,
        nevents,
    );

    if u64::from(nsubscriptions) > wasm32::__wasi_filesize_t::max_value() {
        return Err(Error::EINVAL);
    }

    enc_pointee(memory, nevents, 0)?;

    let input_slice = dec_slice_of::<wasm32::__wasi_subscription_t>(memory, input, nsubscriptions)?;
    let input: Vec<_> = input_slice.iter().map(dec_subscription).collect();
    // Is this actually needed??
    let output_slice = dec_slice_of_mut::<wasm32::__wasi_event_t>(memory, output, nsubscriptions)?;
    let timeout = input
        .iter()
        .filter_map(|event| match event {
            Ok(event) if event.type_ == wasm32::__WASI_EVENTTYPE_CLOCK => Some(ClockEventData {
                delay: wasi_clock_to_relative_ns_delay(unsafe { event.u.clock }).ok()? / 1_000_000,
                userdata: event.userdata,
            }),
            _ => None,
        })
        .min_by_key(|event| event.delay);

    let fd_events: Vec<_> = input
        .iter()
        .filter_map(|event| match event {
            Ok(event)
                if event.type_ == wasm32::__WASI_EVENTTYPE_FD_READ
                    || event.type_ == wasm32::__WASI_EVENTTYPE_FD_WRITE =>
            {
                Some(FdEventData {
                    fd: unsafe { event.u.fd_readwrite.fd },
                    type_: event.type_,
                    userdata: event.userdata,
                })
            }
            _ => None,
        })
        .collect();

    let events = hostcalls_impl::poll_oneoff(fd_events, timeout)?;
    let events_count = events.len();
    let mut output_slice_cur = output_slice.iter_mut();
    for event in events {
        *output_slice_cur.next().unwrap() = enc_event(event);
    }

    trace!("     | *nevents={:?}", events_count);

    enc_pointee(memory, nevents, events_count)
}

fn wasi_clock_to_relative_ns_delay(
    wasi_clock: host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
) -> Result<u128> {
    if wasi_clock.flags != wasm32::__WASI_SUBSCRIPTION_CLOCK_ABSTIME {
        return Ok(u128::from(wasi_clock.timeout));
    }
    let now: u128 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| Error::ENOTCAPABLE)?
        .as_nanos();
    let deadline = u128::from(wasi_clock.timeout);
    Ok(deadline.saturating_sub(now))
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct ClockEventData {
    pub delay: u128,
    pub userdata: host::__wasi_userdata_t,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct FdEventData {
    pub fd: u32,
    pub type_: host::__wasi_eventtype_t,
    pub userdata: host::__wasi_userdata_t,
}

pub(crate) fn sched_yield() -> Result<()> {
    trace!("sched_yield()");

    std::thread::yield_now();

    Ok(())
}
