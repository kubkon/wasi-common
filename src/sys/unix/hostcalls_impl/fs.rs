#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use super::fs_helpers::*;
use crate::ctx::WasiCtx;
use crate::fdentry::{Descriptor, FdEntry};
use crate::sys::errno_from_host;
use crate::sys::fdentry_impl::determine_type_rights;
use crate::sys::host_impl;
use crate::{host, wasm32, Result};
use nix::libc::{self, c_long, c_void, off_t};
use std::ffi::CString;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::os::unix::prelude::{AsRawFd, FromRawFd};

pub(crate) fn fd_pread(
    file: &File,
    buf: &mut [u8],
    offset: host::__wasi_filesize_t,
) -> Result<usize> {
    file.read_at(buf, offset)
        .map_err(|e| e.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}

pub(crate) fn fd_pwrite(file: &File, buf: &[u8], offset: host::__wasi_filesize_t) -> Result<usize> {
    file.write_at(buf, offset)
        .map_err(|e| e.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}

pub(crate) fn fd_fdstat_get(fd: &File) -> Result<host::__wasi_fdflags_t> {
    use nix::fcntl::{fcntl, OFlag, F_GETFL};
    match fcntl(fd.as_raw_fd(), F_GETFL).map(OFlag::from_bits_truncate) {
        Ok(flags) => Ok(host_impl::fdflags_from_nix(flags)),
        Err(e) => Err(host_impl::errno_from_nix(e.as_errno().unwrap())),
    }
}

pub(crate) fn fd_fdstat_set_flags(fd: &File, fdflags: host::__wasi_fdflags_t) -> Result<()> {
    let nix_flags = host_impl::nix_from_fdflags(fdflags);
    match nix::fcntl::fcntl(fd.as_raw_fd(), nix::fcntl::F_SETFL(nix_flags)) {
        Ok(_) => Ok(()),
        Err(e) => Err(host_impl::errno_from_nix(e.as_errno().unwrap())),
    }
}

pub(crate) fn fd_advise(
    file: &File,
    advice: host::__wasi_advice_t,
    offset: host::__wasi_filesize_t,
    len: host::__wasi_filesize_t,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let host_advice = match advice {
            host::__WASI_ADVICE_DONTNEED => libc::POSIX_FADV_DONTNEED,
            host::__WASI_ADVICE_SEQUENTIAL => libc::POSIX_FADV_SEQUENTIAL,
            host::__WASI_ADVICE_WILLNEED => libc::POSIX_FADV_DONTNEED,
            host::__WASI_ADVICE_NOREUSE => libc::POSIX_FADV_NOREUSE,
            host::__WASI_ADVICE_RANDOM => libc::POSIX_FADV_RANDOM,
            host::__WASI_ADVICE_NORMAL => libc::POSIX_FADV_NORMAL,
            _ => return Err(host::__WASI_EINVAL),
        };
        let res = unsafe {
            libc::posix_fadvise(file.as_raw_fd(), offset as off_t, len as off_t, host_advice)
        };
        if res != 0 {
            return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (file, offset, len);
        match advice {
            host::__WASI_ADVICE_DONTNEED
            | host::__WASI_ADVICE_SEQUENTIAL
            | host::__WASI_ADVICE_WILLNEED
            | host::__WASI_ADVICE_NOREUSE
            | host::__WASI_ADVICE_RANDOM
            | host::__WASI_ADVICE_NORMAL => {}
            _ => return Err(host::__WASI_EINVAL),
        }
    }

    Ok(())
}

pub(crate) fn path_create_directory(dirfd: &File, path: &str) -> Result<()> {
    use nix::libc::mkdirat;

    let (dir, path) = match path_get(dirfd, 0, path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let path_cstr = CString::new(path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

    // nix doesn't expose mkdirat() yet
    match unsafe { mkdirat(dir.as_raw_fd(), path_cstr.as_ptr(), 0o777) } {
        0 => Ok(()),
        _ => Err(host_impl::errno_from_nix(nix::errno::Errno::last())),
    }
}

pub(crate) fn path_link(
    old_dirfd: &File,
    new_dirfd: &File,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    use nix::libc::linkat;
    let (old_dir, old_path) = match path_get(old_dirfd, 0, old_path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let (new_dir, new_path) = match path_get(new_dirfd, 0, new_path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let old_path_cstr = CString::new(old_path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;
    let new_path_cstr = CString::new(new_path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

    // Not setting AT_SYMLINK_FOLLOW fails on most filesystems
    let atflags = libc::AT_SYMLINK_FOLLOW;
    let res = unsafe {
        linkat(
            old_dir.as_raw_fd(),
            old_path_cstr.as_ptr(),
            new_dir.as_raw_fd(),
            new_path_cstr.as_ptr(),
            atflags,
        )
    };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn path_open(
    ctx: &WasiCtx,
    dirfd: host::__wasi_fd_t,
    dirflags: host::__wasi_lookupflags_t,
    path: &str,
    oflags: host::__wasi_oflags_t,
    read: bool,
    write: bool,
    mut needed_base: host::__wasi_rights_t,
    mut needed_inheriting: host::__wasi_rights_t,
    fs_flags: host::__wasi_fdflags_t,
) -> Result<FdEntry> {
    use nix::errno::Errno;
    use nix::fcntl::{openat, AtFlags, OFlag};
    use nix::sys::stat::{fstatat, Mode, SFlag};

    let mut nix_all_oflags = if read && write {
        OFlag::O_RDWR
    } else if write {
        OFlag::O_WRONLY
    } else {
        OFlag::O_RDONLY
    };

    // on non-Capsicum systems, we always want nofollow
    nix_all_oflags.insert(OFlag::O_NOFOLLOW);

    // convert open flags
    let nix_oflags = host_impl::nix_from_oflags(oflags);
    nix_all_oflags.insert(nix_oflags);
    if nix_all_oflags.contains(OFlag::O_CREAT) {
        needed_base |= host::__WASI_RIGHT_PATH_CREATE_FILE;
    }
    if nix_all_oflags.contains(OFlag::O_TRUNC) {
        needed_base |= host::__WASI_RIGHT_PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    nix_all_oflags.insert(host_impl::nix_from_fdflags(fs_flags));
    if nix_all_oflags.contains(OFlag::O_DSYNC) {
        needed_inheriting |= host::__WASI_RIGHT_FD_DATASYNC;
    }
    if nix_all_oflags.intersects(host_impl::O_RSYNC | OFlag::O_SYNC) {
        needed_inheriting |= host::__WASI_RIGHT_FD_SYNC;
    }

    let dirfe = ctx.get_fd_entry(dirfd, needed_base, needed_inheriting)?;
    let dirfd = match &*dirfe.fd_object.descriptor {
        Descriptor::File(f) => f,
        _ => return Err(host::__WASI_EBADF),
    };

    let (dir, path) = match path_get(dirfd, dirflags, path, nix_oflags.contains(OFlag::O_CREAT)) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };

    // Call openat. Use mode 0o666 so that we follow whatever the user's
    // umask is, but don't set the executable flag, because it isn't yet
    // meaningful for WASI programs to create executable files.
    let new_fd = match openat(
        dir.as_raw_fd(),
        path.as_str(),
        nix_all_oflags,
        Mode::from_bits_truncate(0o666),
    ) {
        Ok(fd) => fd,
        Err(e) => {
            match e.as_errno() {
                // Linux returns ENXIO instead of EOPNOTSUPP when opening a socket
                Some(Errno::ENXIO) => {
                    if let Ok(stat) =
                        fstatat(dir.as_raw_fd(), path.as_str(), AtFlags::AT_SYMLINK_NOFOLLOW)
                    {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFSOCK) {
                            return Err(host::__WASI_ENOTSUP);
                        } else {
                            return Err(host::__WASI_ENXIO);
                        }
                    } else {
                        return Err(host::__WASI_ENXIO);
                    }
                }
                // Linux returns ENOTDIR instead of ELOOP when using O_NOFOLLOW|O_DIRECTORY
                // on a symlink.
                Some(Errno::ENOTDIR)
                    if !(nix_all_oflags & (OFlag::O_NOFOLLOW | OFlag::O_DIRECTORY)).is_empty() =>
                {
                    if let Ok(stat) =
                        fstatat(dir.as_raw_fd(), path.as_str(), AtFlags::AT_SYMLINK_NOFOLLOW)
                    {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFLNK) {
                            return Err(host::__WASI_ELOOP);
                        }
                    }
                    return Err(host::__WASI_ENOTDIR);
                }
                // FreeBSD returns EMLINK instead of ELOOP when using O_NOFOLLOW on
                // a symlink.
                Some(Errno::EMLINK) if !(nix_all_oflags & OFlag::O_NOFOLLOW).is_empty() => {
                    return Err(host::__WASI_ELOOP);
                }
                Some(e) => return Err(host_impl::errno_from_nix(e)),
                None => return Err(host::__WASI_ENOSYS),
            }
        }
    };

    // Determine the type of the new file descriptor and which rights contradict with this type
    let file = unsafe { File::from_raw_fd(new_fd) };
    match determine_type_rights(&file) {
        Err(e) => Err(e),
        Ok((_ty, max_base, max_inheriting)) => {
            let mut fe = FdEntry::from(file)?;
            fe.rights_base &= max_base;
            fe.rights_inheriting &= max_inheriting;
            Ok(fe)
        }
    }
}

pub(crate) fn fd_readdir(
    fd: &File,
    host_buf: &mut [u8],
    cookie: host::__wasi_dircookie_t,
) -> Result<usize> {
    use libc::{dirent, fdopendir, memcpy, readdir_r, seekdir};

    let host_buf_ptr = host_buf.as_mut_ptr();
    let host_buf_len = host_buf.len();
    let dir = unsafe { fdopendir(fd.as_raw_fd()) };
    if dir.is_null() {
        return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
    }
    if cookie != wasm32::__WASI_DIRCOOKIE_START {
        unsafe { seekdir(dir, cookie as c_long) };
    }
    let mut entry_buf = unsafe { std::mem::uninitialized::<dirent>() };
    let mut left = host_buf_len;
    let mut host_buf_offset: usize = 0;
    while left > 0 {
        let mut host_entry: *mut dirent = std::ptr::null_mut();
        let res = unsafe { readdir_r(dir, &mut entry_buf, &mut host_entry) };
        if res == -1 {
            return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
        }
        if host_entry.is_null() {
            break;
        }
        let entry: wasm32::__wasi_dirent_t =
            match host_impl::dirent_from_host(&unsafe { *host_entry }) {
                Ok(entry) => entry,
                Err(e) => return Err(e),
            };
        let name_len = entry.d_namlen as usize;
        let required_space = std::mem::size_of_val(&entry) + name_len;
        if required_space > left {
            break;
        }
        unsafe {
            let ptr = host_buf_ptr.offset(host_buf_offset as isize) as *mut c_void
                as *mut wasm32::__wasi_dirent_t;
            *ptr = entry;
        }
        host_buf_offset += std::mem::size_of_val(&entry);
        let name_ptr = unsafe { *host_entry }.d_name.as_ptr();
        unsafe {
            memcpy(
                host_buf_ptr.offset(host_buf_offset as isize) as *mut _,
                name_ptr as *const _,
                name_len,
            )
        };
        host_buf_offset += name_len;
        left -= required_space;
    }
    Ok(host_buf_len - left)
}

pub(crate) fn path_readlink(dirfd: &File, path: &str, buf: &mut [u8]) -> Result<usize> {
    use nix::errno::Errno;

    let (dir, path) = match path_get(dirfd, 0, path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let path_cstr = CString::new(path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

    // Linux requires that the buffer size is positive, whereas POSIX does not.
    // Use a fake buffer to store the results if the size is zero.
    // TODO: instead of using raw libc::readlinkat call here, this should really
    // be fixed in `nix` crate
    let fakebuf: &mut [u8] = &mut [0];
    let buf_len = buf.len();
    let len = unsafe {
        libc::readlinkat(
            dir.as_raw_fd(),
            path_cstr.as_ptr() as *const libc::c_char,
            if buf_len == 0 {
                fakebuf.as_mut_ptr()
            } else {
                buf.as_mut_ptr()
            } as *mut libc::c_char,
            if buf_len == 0 { fakebuf.len() } else { buf_len },
        )
    };

    if len < 0 {
        Err(host_impl::errno_from_nix(Errno::last()))
    } else {
        let len = len as usize;
        Ok(if len < buf_len { len } else { buf_len })
    }
}

pub(crate) fn path_rename(
    old_dirfd: &File,
    old_path: &str,
    new_dirfd: &File,
    new_path: &str,
) -> Result<()> {
    use nix::libc::renameat;

    let (old_dir, old_path) = match path_get(old_dirfd, 0, old_path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let (new_dir, new_path) = match path_get(new_dirfd, 0, new_path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let old_path_cstr = CString::new(old_path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;
    let new_path_cstr = CString::new(new_path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

    let res = unsafe {
        renameat(
            old_dir.as_raw_fd(),
            old_path_cstr.as_ptr(),
            new_dir.as_raw_fd(),
            new_path_cstr.as_ptr(),
        )
    };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn fd_filestat_get(fd: &File) -> Result<host::__wasi_filestat_t> {
    use nix::sys::stat::fstat;
    match fstat(fd.as_raw_fd()) {
        Err(e) => Err(host_impl::errno_from_nix(e.as_errno().unwrap())),
        Ok(filestat) => Ok(host_impl::filestat_from_nix(filestat)?),
    }
}

pub(crate) fn fd_filestat_set_times(
    fd: &File,
    st_atim: host::__wasi_timestamp_t,
    mut st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<()> {
    use nix::sys::time::{TimeSpec, TimeValLike};

    if fst_flags & host::__WASI_FILESTAT_SET_MTIM_NOW != 0 {
        let clock_id = libc::CLOCK_REALTIME;
        let mut timespec = unsafe { std::mem::uninitialized::<libc::timespec>() };
        let res = unsafe { libc::clock_gettime(clock_id, &mut timespec as *mut libc::timespec) };
        if res != 0 {
            return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
        }
        let time_ns = match (timespec.tv_sec as host::__wasi_timestamp_t)
            .checked_mul(1_000_000_000)
            .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        {
            Some(time_ns) => time_ns,
            None => return Err(host::__WASI_EOVERFLOW),
        };
        st_mtim = time_ns;
    }
    let ts_atime = match fst_flags {
        f if f & host::__WASI_FILESTAT_SET_ATIM_NOW != 0 => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_now(),
        },
        f if f & host::__WASI_FILESTAT_SET_ATIM != 0 => {
            *TimeSpec::nanoseconds(st_atim as i64).as_ref()
        }
        _ => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_omit(),
        },
    };
    let ts_mtime = *TimeSpec::nanoseconds(st_mtim as i64).as_ref();
    let times = [ts_atime, ts_mtime];
    let res = unsafe { libc::futimens(fd.as_raw_fd(), times.as_ptr()) };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn fd_filestat_set_size(fd: &File, st_size: host::__wasi_filesize_t) -> Result<()> {
    use nix::unistd::ftruncate;
    ftruncate(fd.as_raw_fd(), st_size as off_t)
        .map_err(|e| host_impl::errno_from_nix(e.as_errno().unwrap()))
}

pub(crate) fn path_filestat_get(
    dirfd: &File,
    dirflags: host::__wasi_lookupflags_t,
    path: &str,
) -> Result<host::__wasi_filestat_t> {
    use nix::fcntl::AtFlags;
    use nix::sys::stat::fstatat;

    let (dir, path) = match path_get(dirfd, dirflags, path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let atflags = match dirflags {
        0 => AtFlags::empty(),
        _ => AtFlags::AT_SYMLINK_NOFOLLOW,
    };

    match fstatat(dir.as_raw_fd(), path.as_str(), atflags) {
        Err(e) => Err(host_impl::errno_from_nix(e.as_errno().unwrap())),
        Ok(filestat) => Ok(host_impl::filestat_from_nix(filestat)?),
    }
}

pub(crate) fn path_filestat_set_times(
    dirfd: &File,
    dirflags: host::__wasi_lookupflags_t,
    path: &str,
    st_atim: host::__wasi_timestamp_t,
    mut st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<()> {
    use nix::sys::time::{TimeSpec, TimeValLike};

    let (dir, path) = match path_get(dirfd, dirflags, &path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let atflags = match dirflags {
        wasm32::__WASI_LOOKUP_SYMLINK_FOLLOW => 0,
        _ => libc::AT_SYMLINK_NOFOLLOW,
    };
    if fst_flags & host::__WASI_FILESTAT_SET_MTIM_NOW != 0 {
        let clock_id = libc::CLOCK_REALTIME;
        let mut timespec = unsafe { std::mem::uninitialized::<libc::timespec>() };
        let res = unsafe { libc::clock_gettime(clock_id, &mut timespec as *mut libc::timespec) };
        if res != 0 {
            return Err(host_impl::errno_from_nix(nix::errno::Errno::last()));
        }
        let time_ns = match (timespec.tv_sec as host::__wasi_timestamp_t)
            .checked_mul(1_000_000_000)
            .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        {
            Some(time_ns) => time_ns,
            None => return Err(host::__WASI_EOVERFLOW),
        };
        st_mtim = time_ns;
    }
    let ts_atime = match fst_flags {
        f if f & host::__WASI_FILESTAT_SET_ATIM_NOW != 0 => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_now(),
        },
        f if f & host::__WASI_FILESTAT_SET_ATIM != 0 => {
            *TimeSpec::nanoseconds(st_atim as i64).as_ref()
        }
        _ => libc::timespec {
            tv_sec: 0,
            tv_nsec: utime_omit(),
        },
    };
    let ts_mtime = *TimeSpec::nanoseconds(st_mtim as i64).as_ref();
    let times = [ts_atime, ts_mtime];

    let path_cstr = CString::new(path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

    let res =
        unsafe { libc::utimensat(dir.as_raw_fd(), path_cstr.as_ptr(), times.as_ptr(), atflags) };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn path_symlink(dirfd: &File, old_path: &str, new_path: &str) -> Result<()> {
    use nix::libc::symlinkat;

    let (dir, new_path) = match path_get(dirfd, 0, new_path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let old_path_cstr = CString::new(old_path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;
    let new_path_cstr = CString::new(new_path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

    let res = unsafe {
        symlinkat(
            old_path_cstr.as_ptr(),
            dir.as_raw_fd(),
            new_path_cstr.as_ptr(),
        )
    };
    if res != 0 {
        Err(host_impl::errno_from_nix(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

pub(crate) fn path_unlink_file(dirfd: &File, path: &str) -> Result<()> {
    use nix::errno;
    use nix::libc::unlinkat;

    let (dir, path) = match path_get(dirfd, 0, path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let path_cstr = CString::new(path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

    // nix doesn't expose unlinkat() yet
    match unsafe { unlinkat(dir.as_raw_fd(), path_cstr.as_ptr(), 0) } {
        0 => Ok(()),
        _ => {
            let mut e = errno::Errno::last();

            #[cfg(not(linux))]
            {
                // Non-Linux implementations may return EPERM when attempting to remove a
                // directory without REMOVEDIR. While that's what POSIX specifies, it's
                // less useful. Adjust this to EISDIR. It doesn't matter that this is not
                // atomic with the unlinkat, because if the file is removed and a directory
                // is created before fstatat sees it, we're racing with that change anyway
                // and unlinkat could have legitimately seen the directory if the race had
                // turned out differently.
                use nix::fcntl::AtFlags;
                use nix::sys::stat::{fstatat, SFlag};

                if e == errno::Errno::EPERM {
                    if let Ok(stat) =
                        fstatat(dir.as_raw_fd(), path.as_str(), AtFlags::AT_SYMLINK_NOFOLLOW)
                    {
                        if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::S_IFDIR) {
                            e = errno::Errno::EISDIR;
                        }
                    } else {
                        e = errno::Errno::last();
                    }
                }
            }

            Err(host_impl::errno_from_nix(e))
        }
    }
}

pub(crate) fn path_remove_directory(dirfd: &File, path: &str) -> Result<()> {
    use nix::errno;
    use nix::libc::{unlinkat, AT_REMOVEDIR};

    let (dir, path) = match path_get(dirfd, 0, path, false) {
        Ok((dir, path)) => (dir, path),
        Err(e) => return Err(e),
    };
    let path_cstr = CString::new(path.as_bytes()).map_err(|_| host::__WASI_EILSEQ)?;

    // nix doesn't expose unlinkat() yet
    match unsafe { unlinkat(dir.as_raw_fd(), path_cstr.as_ptr(), AT_REMOVEDIR) } {
        0 => Ok(()),
        _ => Err(host_impl::errno_from_nix(errno::Errno::last())),
    }
}
