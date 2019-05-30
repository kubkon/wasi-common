#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(unused)]
use super::fdentry::{determine_type_rights, FdEntry};
use super::fs_helpers::*;
use super::host_impl;

use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::{host, wasm32};

use std::cmp;
use std::ffi::{OsStr, OsString};
use std::os::windows::prelude::{FromRawHandle, OsStrExt, OsStringExt};
use wasi_common_cbindgen::wasi_common_cbindgen;

#[wasi_common_cbindgen]
pub fn fd_close(wasi_ctx: &mut WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    if let Some(fdent) = wasi_ctx.fds.get(&fd) {
        // can't close preopened files
        if fdent.preopen_path.is_some() {
            return wasm32::__WASI_ENOTSUP;
        }
    }
    if let Some(mut fdent) = wasi_ctx.fds.remove(&fd) {
        fdent.fd_object.needs_close = false;
        match winx::handle::close(fdent.fd_object.raw_handle) {
            Ok(_) => wasm32::__WASI_ESUCCESS,
            Err(e) => host_impl::errno_from_win(e),
        }
    } else {
        wasm32::__WASI_EBADF
    }
}

#[wasi_common_cbindgen]
pub fn fd_datasync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_datasync")
}

#[wasi_common_cbindgen]
pub fn fd_pread(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    offset: wasm32::__wasi_filesize_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_pread")
}

#[wasi_common_cbindgen]
pub fn fd_pwrite(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    offset: wasm32::__wasi_filesize_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_pwrite")
}

#[wasi_common_cbindgen]
pub fn fd_read(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    let mut iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let fe = match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_FD_READ.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };

    let mut iovs: Vec<winx::io::IoVecMut> = iovs
        .iter_mut()
        .map(|iov| unsafe { host_impl::iovec_to_win_mut(iov) })
        .collect();

    let host_nread = match winx::io::readv(fe.fd_object.raw_handle, &mut iovs) {
        Ok(len) => len,
        Err(e) => return host_impl::errno_from_win(e),
    };

    if host_nread == 0 {
        // we hit eof, so remove the fdentry from the context
        let mut fe = wasi_ctx.fds.remove(&fd).expect("file entry is still there");
        fe.fd_object.needs_close = false;
    }

    enc_usize_byref(memory, nread, host_nread)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

#[wasi_common_cbindgen]
pub fn fd_renumber(
    wasi_ctx: &mut WasiCtx,
    from: wasm32::__wasi_fd_t,
    to: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_renumber")
}

#[wasi_common_cbindgen]
pub fn fd_seek(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filedelta_t,
    whence: wasm32::__wasi_whence_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_seek")
}

#[wasi_common_cbindgen]
pub fn fd_tell(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_tell")
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    fdstat_ptr: wasm32::uintptr_t, // *mut wasm32::__wasi_fdstat_t
) -> wasm32::__wasi_errno_t {
    let host_fd = dec_fd(fd);
    let mut host_fdstat = match dec_fdstat_byref(memory, fdstat_ptr) {
        Ok(host_fdstat) => host_fdstat,
        Err(e) => return enc_errno(e),
    };

    let errno = if let Some(fe) = wasi_ctx.fds.get(&host_fd) {
        host_fdstat.fs_filetype = fe.fd_object.ty;
        host_fdstat.fs_rights_base = fe.rights_base;
        host_fdstat.fs_rights_inheriting = fe.rights_inheriting;

        use winx::file::AccessRight;
        match winx::file::get_file_access_rights(fe.fd_object.raw_handle)
            .map(AccessRight::from_bits_truncate)
        {
            Ok(rights) => {
                host_fdstat.fs_flags = host_impl::fdflags_from_win(rights);
                wasm32::__WASI_ESUCCESS
            }
            Err(e) => host_impl::errno_from_win(e),
        }
    } else {
        wasm32::__WASI_EBADF
    };

    enc_fdstat_byref(memory, fdstat_ptr, host_fdstat)
        .expect("can write back into the pointer we read from");

    errno
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_set_flags(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    fdflags: wasm32::__wasi_fdflags_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_fdstat_set_flags")
}

#[wasi_common_cbindgen]
pub fn fd_fdstat_set_rights(
    wasi_ctx: &mut WasiCtx,
    fd: wasm32::__wasi_fd_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_fdstat_set_rights")
}

#[wasi_common_cbindgen]
pub fn fd_sync(wasi_ctx: &WasiCtx, fd: wasm32::__wasi_fd_t) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_sync")
}

#[wasi_common_cbindgen]
pub fn fd_write(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use winx::io::IoVec;

    let fd = dec_fd(fd);
    let mut iovs = match dec_iovec_slice(memory, iovs_ptr, iovs_len) {
        Ok(iovs) => iovs,
        Err(e) => return enc_errno(e),
    };

    let fe = match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_FD_WRITE.into(), 0) {
        Ok(fe) => fe,
        Err(e) => return enc_errno(e),
    };

    let iovs: Vec<IoVec> = iovs
        .iter()
        .map(|iov| unsafe { host_impl::iovec_to_win(iov) })
        .collect();

    let host_nwritten = match winx::io::writev(fe.fd_object.raw_handle, &iovs) {
        Ok(len) => len,
        Err(e) => return host_impl::errno_from_win(e),
    };

    enc_usize_byref(memory, nwritten, host_nwritten)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

#[wasi_common_cbindgen]
pub fn fd_advise(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
    advice: wasm32::__wasi_advice_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_advise")
}

#[wasi_common_cbindgen]
pub fn fd_allocate(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_allocate")
}

#[wasi_common_cbindgen]
pub fn path_create_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_create_directory")
}

#[wasi_common_cbindgen]
pub fn path_link(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_dirfd: wasm32::__wasi_fd_t,
    _old_flags: wasm32::__wasi_lookupflags_t,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    new_dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_link")
}

#[wasi_common_cbindgen]
pub fn path_open(
    wasi_ctx: &mut WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    oflags: wasm32::__wasi_oflags_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
    fs_flags: wasm32::__wasi_fdflags_t,
    fd_out_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    use winx::file::{AccessRight, CreationDisposition, FlagsAndAttributes};

    let dirfd = dec_fd(dirfd);
    let dirflags = dec_lookupflags(dirflags);
    let oflags = dec_oflags(oflags);
    let fs_rights_base = dec_rights(fs_rights_base);
    let fs_rights_inheriting = dec_rights(fs_rights_inheriting);
    let fs_flags = dec_fdflags(fs_flags);

    // which open mode do we need?
    let read = fs_rights_base & (host::__WASI_RIGHT_FD_READ | host::__WASI_RIGHT_FD_READDIR) != 0;
    let write = fs_rights_base
        & (host::__WASI_RIGHT_FD_DATASYNC
            | host::__WASI_RIGHT_FD_WRITE
            | host::__WASI_RIGHT_FD_ALLOCATE
            | host::__WASI_RIGHT_FD_FILESTAT_SET_SIZE)
        != 0;

    let mut win_all_rights = AccessRight::empty();
    if read {
        win_all_rights |= AccessRight::FILE_GENERIC_READ;
    }
    if write {
        win_all_rights |= AccessRight::FILE_GENERIC_WRITE;
    }

    // which rights are needed on the dirfd?
    let mut needed_base = host::__WASI_RIGHT_PATH_OPEN;
    let mut needed_inheriting = fs_rights_base | fs_rights_inheriting;

    // convert open flags
    let (win_create_disp, win_flags_attrs) = host_impl::win_from_oflags(oflags);
    if win_create_disp == CreationDisposition::CREATE_NEW {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    } else if win_create_disp == CreationDisposition::CREATE_ALWAYS {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    } else if win_create_disp == CreationDisposition::TRUNCATE_EXISTING {
        needed_inheriting |= host::__WASI_RIGHT_PATH_FILESTAT_SET_SIZE;
    }

    // // convert file descriptor flags
    // nix_all_oflags.insert(host_impl::nix_from_fdflags(fs_flags));
    // if nix_all_oflags.contains(OFlag::O_DSYNC) {
    //     needed_inheriting |= host::__WASI_RIGHT_FD_DATASYNC;
    // }
    // if nix_all_oflags.intersects(host_impl::O_RSYNC | OFlag::O_SYNC) {
    //     needed_inheriting |= host::__WASI_RIGHT_FD_SYNC;
    // }

    let path = match dec_slice_of::<u8>(memory, path_ptr, path_len) {
        Ok(slice) => {
            OsString::from_wide(&slice.into_iter().map(|&x| x as u16).collect::<Vec<u16>>())
        }
        Err(e) => return enc_errno(e),
    };

    let (dir, path) = match path_get(
        wasi_ctx,
        dirfd,
        dirflags,
        path,
        needed_base,
        needed_inheriting,
        !win_flags_attrs.contains(FlagsAndAttributes::FILE_FLAG_BACKUP_SEMANTICS),
    ) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return enc_errno(e),
    };

    let new_handle =
        match winx::file::openat(dir, &path, win_all_rights, win_create_disp, win_flags_attrs) {
            Ok(handle) => handle,
            Err(e) => return host_impl::errno_from_win(e),
        };

    // Determine the type of the new file descriptor and which rights contradict with this type
    let guest_fd = match unsafe { determine_type_rights(new_handle) } {
        Err(e) => {
            // if `close` fails, note it but do not override the underlying errno
            winx::handle::close(new_handle).unwrap_or_else(|e| {
                dbg!(e);
            });
            return enc_errno(e);
        }
        Ok((_ty, max_base, max_inheriting)) => {
            let mut fe = unsafe { FdEntry::from_raw_handle(new_handle) };
            fe.rights_base &= max_base;
            fe.rights_inheriting &= max_inheriting;
            match wasi_ctx.insert_fd_entry(fe) {
                Ok(fd) => fd,
                Err(e) => return enc_errno(e),
            }
        }
    };

    enc_fd_byref(memory, fd_out_ptr, guest_fd)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

#[wasi_common_cbindgen]
pub fn fd_readdir(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    buf: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
    cookie: wasm32::__wasi_dircookie_t,
    buf_used: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_readdir")
}

#[wasi_common_cbindgen]
pub fn path_readlink(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
    buf_used: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_readlink")
}

#[wasi_common_cbindgen]
pub fn path_rename(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_dirfd: wasm32::__wasi_fd_t,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    new_dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_rename")
}

#[wasi_common_cbindgen]
pub fn fd_filestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_filestat_get")
}

#[wasi_common_cbindgen]
pub fn fd_filestat_set_times(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_filestat_set_times")
}

#[wasi_common_cbindgen]
pub fn fd_filestat_set_size(
    wasi_ctx: &WasiCtx,
    fd: wasm32::__wasi_fd_t,
    st_size: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("fd_filestat_set_size")
}

#[wasi_common_cbindgen]
pub fn path_filestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_filestat_get")
}

#[wasi_common_cbindgen]
pub fn path_filestat_set_times(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_filestat_set_times")
}

#[wasi_common_cbindgen]
pub fn path_symlink(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_symlink")
}

#[wasi_common_cbindgen]
pub fn path_unlink_file(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_unlink_file")
}

#[wasi_common_cbindgen]
pub fn path_remove_directory(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    unimplemented!("path_remove_directory")
}

#[wasi_common_cbindgen]
pub fn fd_prestat_get(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    prestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);
    // TODO: is this the correct right for this?
    match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY {
                    return wasm32::__WASI_ENOTDIR;
                }
                enc_prestat_byref(
                    memory,
                    prestat_ptr,
                    host::__wasi_prestat_t {
                        pr_type: host::__WASI_PREOPENTYPE_DIR,
                        u: host::__wasi_prestat_t___wasi_prestat_u {
                            dir: host::__wasi_prestat_t___wasi_prestat_u___wasi_prestat_u_dir_t {
                                // TODO: clean up
                                pr_name_len: po_path.as_os_str().encode_wide().count() * 2,
                            },
                        },
                    },
                )
                .map(|_| wasm32::__WASI_ESUCCESS)
                .unwrap_or_else(|e| e)
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}

#[wasi_common_cbindgen]
pub fn fd_prestat_dir_name(
    wasi_ctx: &WasiCtx,
    memory: &mut [u8],
    fd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let fd = dec_fd(fd);

    match wasi_ctx.get_fd_entry(fd, host::__WASI_RIGHT_PATH_OPEN.into(), 0) {
        Ok(fe) => {
            if let Some(po_path) = &fe.preopen_path {
                if fe.fd_object.ty != host::__WASI_FILETYPE_DIRECTORY {
                    return wasm32::__WASI_ENOTDIR;
                }
                // TODO: clean up
                let path_bytes = &po_path
                    .as_os_str()
                    .encode_wide()
                    .map(u16::to_le_bytes)
                    .fold(Vec::new(), |mut acc, bytes| {
                        acc.extend_from_slice(&bytes);
                        acc
                    });
                if path_bytes.len() > dec_usize(path_len) {
                    return wasm32::__WASI_ENAMETOOLONG;
                }
                enc_slice_of(memory, path_bytes, path_ptr)
                    .map(|_| wasm32::__WASI_ESUCCESS)
                    .unwrap_or_else(|e| e)
            } else {
                wasm32::__WASI_ENOTSUP
            }
        }
        Err(e) => enc_errno(e),
    }
}
