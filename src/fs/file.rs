use crate::fs::{ctx, error::wasi_errno_to_io_error};
use crate::{host, hostcalls};
use std::io;

/// A reference to an open file on the filesystem.
pub struct File {
    fd: host::__wasi_fd_t,
}

impl File {
    /// Constructs a new instance of Self from the given raw WASI file descriptor.
    pub unsafe fn from_raw_wasi_fd(fd: host::__wasi_fd_t) -> Self {
        Self { fd }
    }

    // TODO: functions to implement: sync_all, sync_data, set_len, metadata
}

impl Drop for File {
    fn drop(&mut self) {
        // Note that errors are ignored when closing a file descriptor. The
        // reason for this is that if an error occurs we don't actually know if
        // the file descriptor was closed or not, and if we retried (for
        // something like EINTR), we might close another valid file descriptor
        // opened after we closed ours.
        let _ = hostcalls::fd_close(&mut ctx::CONTEXT, self.fd);
    }
}

impl io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let iov = [host::__wasi_iovec_t {
            buf: buf.as_mut_ptr() as *mut core::ffi::c_void,
            buf_len: buf.len(),
        }];
        let mut nread = 0;

        wasi_errno_to_io_error(hostcalls::fd_read(
            &mut ctx::CONTEXT,
            self.fd,
            &iov,
            1,
            &mut nread,
        ))?;

        Ok(nread)
    }
}

// TODO: traits to implement: Write, Seek, FileExt
