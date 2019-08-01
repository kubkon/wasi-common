#![allow(non_camel_case_types)]
#![allow(unused)]
use super::fs_helpers::*;
use crate::ctx::WasiCtx;
use crate::fdentry::FdEntry;
use crate::helpers::systemtime_to_timestamp;
use crate::sys::fdentry_impl::determine_type_rights;
use crate::sys::host_impl;
use crate::sys::{errno_from_host, errno_from_ioerror};
use crate::{host, Result};
use std::convert::TryInto;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Seek, SeekFrom};
use std::os::windows::fs::{FileExt, OpenOptionsExt};
use std::os::windows::prelude::{AsRawHandle, FromRawHandle};
use std::path::Path;

fn read_at(mut file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    // get current cursor position
    let cur_pos = file.seek(SeekFrom::Current(0))?;
    // perform a seek read by a specified offset
    let nread = file.seek_read(buf, offset)?;
    // rewind the cursor back to the original position
    file.seek(SeekFrom::Start(cur_pos))?;
    Ok(nread)
}

fn write_at(mut file: &File, buf: &[u8], offset: u64) -> io::Result<usize> {
    // get current cursor position
    let cur_pos = file.seek(SeekFrom::Current(0))?;
    // perform a seek write by a specified offset
    let nwritten = file.seek_write(buf, offset)?;
    // rewind the cursor back to the original position
    file.seek(SeekFrom::Start(cur_pos))?;
    Ok(nwritten)
}

pub(crate) fn fd_pread(
    file: &File,
    buf: &mut [u8],
    offset: host::__wasi_filesize_t,
) -> Result<usize> {
    read_at(file, buf, offset)
        .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}

pub(crate) fn fd_pwrite(file: &File, buf: &[u8], offset: host::__wasi_filesize_t) -> Result<usize> {
    write_at(file, buf, offset)
        .map_err(|err| err.raw_os_error().map_or(host::__WASI_EIO, errno_from_host))
}

pub(crate) fn fd_fdstat_get(fd: &File) -> Result<host::__wasi_fdflags_t> {
    use winx::file::AccessMode;
    winx::file::get_file_access_mode(fd.as_raw_handle())
        .map(host_impl::fdflags_from_win)
        .map_err(host_impl::errno_from_win)
}

pub(crate) fn fd_fdstat_set_flags(fd: &File, fdflags: host::__wasi_fdflags_t) -> Result<()> {
    unimplemented!("fd_fdstat_set_flags")
}

pub(crate) fn fd_advise(
    file: &File,
    advice: host::__wasi_advice_t,
    offset: host::__wasi_filesize_t,
    len: host::__wasi_filesize_t,
) -> Result<()> {
    unimplemented!("fd_advise")
}

pub(crate) fn path_create_directory(dirfd: File, path: String) -> Result<()> {
    let path = concatenate(&dirfd, Path::new(&path))?;
    std::fs::create_dir(path).map_err(errno_from_ioerror)
}

pub(crate) fn path_link(
    old_dirfd: File,
    new_dirfd: File,
    old_path: String,
    new_path: String,
) -> Result<()> {
    unimplemented!("path_link")
}

pub(crate) fn path_open(
    dirfd: File,
    path: String,
    read: bool,
    write: bool,
    oflags: host::__wasi_oflags_t,
    fdflags: host::__wasi_fdflags_t,
) -> Result<File> {
    use winx::file::{AccessMode, CreationDisposition, Flags};

    let mut access_mode = AccessMode::READ_CONTROL;
    if read {
        access_mode.insert(AccessMode::FILE_GENERIC_READ);
    }
    if write {
        access_mode.insert(AccessMode::FILE_GENERIC_WRITE);
    }

    let mut flags = Flags::FILE_FLAG_BACKUP_SEMANTICS;

    // convert open flags
    let mut opts = OpenOptions::new();
    match host_impl::win_from_oflags(oflags) {
        CreationDisposition::CREATE_ALWAYS => {
            opts.create(true).append(true);
        }
        CreationDisposition::CREATE_NEW => {
            opts.create_new(true).write(true);
        }
        CreationDisposition::TRUNCATE_EXISTING => {
            opts.truncate(true);
        }
        _ => {}
    }

    // convert file descriptor flags
    let (add_access_mode, add_flags) = host_impl::win_from_fdflags(fdflags);
    access_mode.insert(add_access_mode);
    flags.insert(add_flags);

    let path = concatenate(&dirfd, Path::new(&path))?;

    // check if we are trying to open a file as a dir
    if path.is_file() && oflags & host::__WASI_O_DIRECTORY != 0 {
        return Err(host::__WASI_ENOTDIR);
    }

    // check if we are trying to open a symlink
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(host::__WASI_ELOOP);
            }
        }
        Err(e) => {
            use winx::winerror::WinError;
            match e.raw_os_error() {
                Some(e) => match WinError::from_u32(e as u32) {
                    WinError::ERROR_PATH_NOT_FOUND | WinError::ERROR_FILE_NOT_FOUND => {
                        // skip
                    }
                    e => return Err(host_impl::errno_from_win(e)),
                },
                None => {
                    log::debug!("Inconvertible OS error: {}", e);
                    return Err(host::__WASI_EIO);
                }
            }
        }
    }

    opts.access_mode(access_mode.bits())
        .custom_flags(flags.bits())
        .open(path)
        .map_err(|e| {
            use winx::winerror::WinError;
            log::debug!("opts error={:?}", e);
            match e.raw_os_error() {
                Some(e) => match WinError::from_u32(e as u32) {
                    WinError::ERROR_INVALID_NAME => {
                        // TODO opening file as a dir
                        host::__WASI_ENOTDIR
                    }
                    e => host_impl::errno_from_win(e),
                },
                None => {
                    log::debug!("Inconvertible OS error: {}", e);
                    host::__WASI_EIO
                }
            }
        })
}

pub(crate) fn fd_readdir(
    fd: &File,
    host_buf: &mut [u8],
    cookie: host::__wasi_dircookie_t,
) -> Result<usize> {
    unimplemented!("fd_readdir")
}

pub(crate) fn path_readlink(dirfd: File, path: String, buf: &mut [u8]) -> Result<usize> {
    unimplemented!("path_readlink")
}

pub(crate) fn path_rename(
    old_dirfd: File,
    old_path: String,
    new_dirfd: File,
    new_path: String,
) -> Result<()> {
    unimplemented!("path_rename")
}

pub(crate) fn num_hardlinks(file: &File, _metadata: &Metadata) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.nNumberOfLinks.into())
}

pub(crate) fn device_id(file: &File, _metadata: &Metadata) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.dwVolumeSerialNumber.into())
}

pub(crate) fn file_serial_no(file: &File, _metadata: &Metadata) -> io::Result<u64> {
    let info = winx::file::get_fileinfo(file)?;
    let high = info.nFileIndexHigh;
    let low = info.nFileIndexLow;
    let no = ((high as u64) << 32) | (low as u64);
    Ok(no)
}

pub(crate) fn change_time(file: &File, _metadata: &Metadata) -> io::Result<i64> {
    winx::file::change_time(file)
}

pub(crate) fn fd_filestat_get_impl(file: &std::fs::File) -> Result<host::__wasi_filestat_t> {
    let metadata = file.metadata().map_err(errno_from_ioerror)?;
    Ok(host::__wasi_filestat_t {
        st_dev: device_id(file, &metadata).map_err(errno_from_ioerror)?,
        st_ino: file_serial_no(file, &metadata).map_err(errno_from_ioerror)?,
        st_nlink: num_hardlinks(file, &metadata)
            .map_err(errno_from_ioerror)?
            .try_into()
            .map_err(|_| host::__WASI_EOVERFLOW)?, // u64 doesn't fit into u32
        st_size: metadata.len(),
        st_atim: metadata
            .accessed()
            .map_err(errno_from_ioerror)
            .and_then(systemtime_to_timestamp)?,
        st_ctim: change_time(file, &metadata)
            .map_err(errno_from_ioerror)?
            .try_into()
            .map_err(|_| host::__WASI_EOVERFLOW)?, // i64 doesn't fit into u64
        st_mtim: metadata
            .modified()
            .map_err(errno_from_ioerror)
            .and_then(systemtime_to_timestamp)?,
        st_filetype: filetype(&metadata).map_err(errno_from_ioerror)?,
    })
}

fn filetype(metadata: &Metadata) -> io::Result<host::__wasi_filetype_t> {
    let ftype = metadata.file_type();
    let ret = if ftype.is_file() {
        host::__WASI_FILETYPE_REGULAR_FILE
    } else if ftype.is_dir() {
        host::__WASI_FILETYPE_DIRECTORY
    } else if ftype.is_symlink() {
        host::__WASI_FILETYPE_SYMBOLIC_LINK
    } else {
        host::__WASI_FILETYPE_UNKNOWN
    };

    Ok(ret)
}

pub(crate) fn fd_filestat_set_times(
    fd: &File,
    st_atim: host::__wasi_timestamp_t,
    mut st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<()> {
    unimplemented!("fd_filestat_set_times")
}

pub(crate) fn fd_filestat_set_size(fd: &File, st_size: host::__wasi_filesize_t) -> Result<()> {
    unimplemented!("fd_filestat_set_size")
}

pub(crate) fn path_filestat_get(
    dirfd: File,
    dirflags: host::__wasi_lookupflags_t,
    path: String,
) -> Result<host::__wasi_filestat_t> {
    unimplemented!("path_filestat_get")
}

pub(crate) fn path_filestat_set_times(
    dirfd: File,
    dirflags: host::__wasi_lookupflags_t,
    path: String,
    st_atim: host::__wasi_timestamp_t,
    mut st_mtim: host::__wasi_timestamp_t,
    fst_flags: host::__wasi_fstflags_t,
) -> Result<()> {
    unimplemented!("path_filestat_set_times")
}

pub(crate) fn path_symlink(dirfd: File, old_path: &str, new_path: String) -> Result<()> {
    use std::os::windows::fs::{symlink_dir, symlink_file};
    let old_path = concatenate(&dirfd, Path::new(&old_path))?;
    let new_path = concatenate(&dirfd, Path::new(&new_path))?;

    if old_path.is_file() {
        // create file symlink
        symlink_file(old_path, new_path).map_err(errno_from_ioerror)
    } else if old_path.is_dir() {
        // create dir symlink
        symlink_dir(old_path, new_path).map_err(errno_from_ioerror)
    } else if !old_path.exists() {
        // OK, so we've been asked to create a dangling symlink
        // AFAIK it is impossible to create a symlink to a
        // nonexistent resource on Windows, or worse, a symlink to itself
        // so, for the moment we'll cheat by creating and then deleting a dir
        // and in-between creating a valid symlink, however, IMHO we should
        // create a wrapper Symlink type which will handle those edge cases
        // virtually, without touching the OS
        // TODO rewrite using custom Symlink type
        create_dangling_symlink(old_path, new_path).map_err(errno_from_ioerror)
    } else {
        Err(host::__WASI_EBADF)
    }
}

fn create_dangling_symlink<P: AsRef<Path>>(old_path: P, new_path: P) -> io::Result<()> {
    use std::fs;
    use std::os::windows::fs::symlink_dir;
    // open a spoof dir
    fs::create_dir(&old_path)?;
    // create dir symlink
    symlink_dir(&old_path, new_path)?;
    // now, delete the spoof dir
    std::fs::remove_dir(old_path)
}

pub(crate) fn path_unlink_file(dirfd: File, path: String) -> Result<()> {
    let path = concatenate(&dirfd, Path::new(&path))?;
    let metadata = std::fs::symlink_metadata(&path).map_err(errno_from_ioerror)?;
    let file_type = metadata.file_type();
    // check if we're actually trying to remove a dir not a file
    if file_type.is_dir() {
        return Err(host::__WASI_EISDIR);
    }
    // check if we're dealing with a symlink
    let is_symlink = file_type.is_symlink();

    if let Err(e) = std::fs::remove_file(&path) {
        use winx::winerror::WinError;
        log::debug!("path_unlink_file error={:?}", e);
        match e.raw_os_error() {
            Some(e) => match WinError::from_u32(e as u32) {
                e @ WinError::ERROR_ACCESS_DENIED => {
                    // if we're dealing with a symlink, try removing a symlink_dir as well
                    // NB this should become much cleaner when FileTypeExt for Windows stabilises
                    // https://doc.rust-lang.org/std/os/windows/fs/trait.FileTypeExt.html#tymethod.is_symlink_dir
                    if is_symlink {
                        if let Err(e) = std::fs::remove_dir(path).map_err(errno_from_ioerror) {
                            return Err(e);
                        }
                    } else {
                        return Err(host_impl::errno_from_win(e));
                    }
                }
                x => return Err(host_impl::errno_from_win(x)),
            },
            None => {
                log::debug!("Inconvertible OS error: {}", e);
                return Err(host::__WASI_EIO);
            }
        }
    }

    Ok(())
}

pub(crate) fn path_remove_directory(dirfd: File, path: String) -> Result<()> {
    let path = concatenate(&dirfd, Path::new(&path))?;
    std::fs::remove_dir(path).map_err(errno_from_ioerror)
}
