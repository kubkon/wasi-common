#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::sys::host_impl;
use crate::{ctx::WasiCtx, host, Error, Result};
use nix::libc::{self, c_int};
use std::cmp;
use std::mem::MaybeUninit;
use std::time::SystemTime;

pub(crate) fn clock_res_get(clock_id: host::__wasi_clockid_t) -> Result<host::__wasi_timestamp_t> {
    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match clock_id {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return Err(Error::EINVAL),
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = MaybeUninit::<libc::timespec>::uninit();
    let res = unsafe { libc::clock_getres(clock_id, timespec.as_mut_ptr()) };
    if res != 0 {
        return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
    }
    let timespec = unsafe { timespec.assume_init() };

    // convert to nanoseconds, returning EOVERFLOW in case of overflow;
    // this is freelancing a bit from the spec but seems like it'll
    // be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map_or(Err(Error::EOVERFLOW), |resolution| {
            // a supported clock can never return zero; this case will probably never get hit, but
            // make sure we follow the spec
            if resolution == 0 {
                Err(Error::EINVAL)
            } else {
                Ok(resolution)
            }
        })
}

pub(crate) fn clock_time_get(clock_id: host::__wasi_clockid_t) -> Result<host::__wasi_timestamp_t> {
    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match clock_id {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return Err(Error::EINVAL),
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = MaybeUninit::<libc::timespec>::uninit();
    let res = unsafe { libc::clock_gettime(clock_id, timespec.as_mut_ptr()) };
    if res != 0 {
        return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
    }
    let timespec = unsafe { timespec.assume_init() };

    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map_or(Err(Error::EOVERFLOW), Ok)
}

pub(crate) fn poll_oneoff(
    wasi_ctx: &WasiCtx,
    input: Vec<host::__wasi_subscription_t>,
) -> Result<Vec<host::__wasi_event_t>> {
    use nix::{
        errno::Errno,
        poll::{poll, PollFd, PollFlags},
    };
    use std::os::unix::prelude::AsRawFd;

    let mut timeout: Option<ClockEventData> = None;
    let mut fd_events = Vec::new();
    for event in input {
        match event.type_ {
            host::__WASI_EVENTTYPE_CLOCK => {
                let clock = unsafe { event.u.clock };
                log::debug!("poll_oneoff event.u.clock = {:?}", clock);
                let delay = wasi_clock_to_relative_ns_delay(clock)? / 1_000_000; // in milliseconds!
                log::debug!("poll_oneoff delay = {:?}ms", delay);
                let current_event = ClockEventData {
                    delay,
                    userdata: event.userdata,
                };
                if let Some(ref mut timeout) = timeout {
                    if current_event.delay < timeout.delay {
                        *timeout = current_event;
                    }
                } else {
                    timeout = Some(current_event);
                }
            }
            host::__WASI_EVENTTYPE_FD_READ => {
                let wasi_fd = unsafe { event.u.fd_readwrite.fd };
                let fd = unsafe {
                    wasi_ctx
                        .get_fd_entry(wasi_fd, host::__WASI_RIGHT_FD_READ, 0)?
                        .fd_object
                        .descriptor
                        .as_raw_fd()
                };
                fd_events.push(FdEventData {
                    fd,
                    type_: event.type_,
                    userdata: event.userdata,
                })
            }
            host::__WASI_EVENTTYPE_FD_WRITE => {
                let wasi_fd = unsafe { event.u.fd_readwrite.fd };
                let fd = unsafe {
                    wasi_ctx
                        .get_fd_entry(wasi_fd, host::__WASI_RIGHT_FD_WRITE, 0)?
                        .fd_object
                        .descriptor
                        .as_raw_fd()
                };
                fd_events.push(FdEventData {
                    fd,
                    type_: event.type_,
                    userdata: event.userdata,
                })
            }
            _ => unreachable!(),
        }
    }

    log::debug!("poll_oneoff timeout = {:?}", timeout);
    log::debug!("poll_oneoff fd_events = {:?}", fd_events);

    if fd_events.is_empty() && timeout.is_none() {
        return Ok(vec![]);
    }

    let mut poll_fds: Vec<_> = fd_events
        .iter()
        .map(|event| {
            let mut flags = PollFlags::empty();
            match event.type_ {
                host::__WASI_EVENTTYPE_FD_READ => flags.insert(PollFlags::POLLIN),
                host::__WASI_EVENTTYPE_FD_WRITE => flags.insert(PollFlags::POLLOUT),
                // An event on a file descriptor can currently only be of type FD_READ or FD_WRITE
                // Nothing else has been defined in the specification, and these are also the only two
                // events we filtered before. If we get something else here, the code has a serious bug.
                _ => unreachable!(),
            };
            PollFd::new(event.fd, flags)
        })
        .collect();

    let timeout = timeout.map(|ClockEventData { delay, userdata }| ClockEventData {
        delay: cmp::min(delay, c_int::max_value() as u128),
        userdata,
    });
    let poll_timeout = timeout.map_or(-1, |timeout| timeout.delay as c_int);
    let ready = loop {
        match poll(&mut poll_fds, poll_timeout) {
            Err(_) => {
                if Errno::last() == Errno::EINTR {
                    continue;
                }
                return Err(host_impl::errno_from_nix(Errno::last()));
            }
            Ok(ready) => break ready as usize,
        }
    };

    if ready == 0 {
        Ok(poll_oneoff_handle_timeout_event(timeout))
    } else {
        let events = fd_events.into_iter().zip(poll_fds.into_iter()).take(ready);
        poll_oneoff_handle_fd_event(events)
    }
}

#[derive(Debug, Copy, Clone)]
struct ClockEventData {
    delay: u128,
    userdata: host::__wasi_userdata_t,
}

#[derive(Debug, Copy, Clone)]
struct FdEventData {
    fd: c_int,
    type_: host::__wasi_eventtype_t,
    userdata: host::__wasi_userdata_t,
}

// define the `fionread()` function, equivalent to `ioctl(fd, FIONREAD, *bytes)`
nix::ioctl_read_bad!(fionread, nix::libc::FIONREAD, c_int);

fn wasi_clock_to_relative_ns_delay(
    wasi_clock: host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
) -> Result<u128> {
    if wasi_clock.flags != host::__WASI_SUBSCRIPTION_CLOCK_ABSTIME {
        return Ok(u128::from(wasi_clock.timeout));
    }
    let now: u128 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| Error::ENOTCAPABLE)?
        .as_nanos();
    let deadline = u128::from(wasi_clock.timeout);
    Ok(deadline.saturating_sub(now))
}

fn poll_oneoff_handle_timeout_event(timeout: Option<ClockEventData>) -> Vec<host::__wasi_event_t> {
    if let Some(ClockEventData { userdata, .. }) = timeout {
        vec![host::__wasi_event_t {
            userdata,
            type_: host::__WASI_EVENTTYPE_CLOCK,
            error: host::__WASI_ESUCCESS,
            u: host::__wasi_event_t___wasi_event_u {
                fd_readwrite: host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                    nbytes: 0,
                    flags: 0,
                },
            },
        }]
    } else {
        // shouldn't happen
        vec![]
    }
}

fn poll_oneoff_handle_fd_event(
    events: impl Iterator<Item = (FdEventData, nix::poll::PollFd)>,
) -> Result<Vec<host::__wasi_event_t>> {
    use nix::poll::PollFlags;
    use std::convert::TryInto;

    let mut output_events = Vec::new();
    for (fd_event, poll_fd) in events {
        log::debug!("poll_oneoff_handle_fd_event fd_event = {:?}", fd_event);
        log::debug!("poll_oneoff_handle_fd_event poll_fd = {:?}", poll_fd);

        let revents = match poll_fd.revents() {
            Some(revents) => revents,
            None => continue,
        };

        log::debug!("poll_oneoff_handle_fd_event revents = {:?}", revents);

        let mut nbytes = 0;
        if fd_event.type_ == host::__WASI_EVENTTYPE_FD_READ {
            let _ = unsafe { fionread(fd_event.fd, &mut nbytes) };
        }

        let output_event = if revents.contains(PollFlags::POLLNVAL) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: host::__WASI_EBADF,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: host::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(PollFlags::POLLERR) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: host::__WASI_EIO,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: host::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(PollFlags::POLLHUP) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: host::__WASI_ESUCCESS,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: host::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(PollFlags::POLLIN) | revents.contains(PollFlags::POLLOUT) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: host::__WASI_ESUCCESS,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: nbytes.try_into()?,
                            flags: 0,
                        },
                },
            }
        } else {
            continue;
        };

        output_events.push(output_event);
    }

    Ok(output_events)
}
