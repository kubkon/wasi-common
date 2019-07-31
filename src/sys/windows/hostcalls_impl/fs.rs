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
    use winx::file::AccessRight;
    match winx::file::get_file_access_rights(fd.as_raw_handle())
        .map(AccessRight::from_bits_truncate)
    {
        Ok(rights) => Ok(host_impl::fdflags_from_win(rights)),
        Err(e) => Err(host_impl::errno_from_win(e)),
    }
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
    use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;
    use winapi::um::winnt::READ_CONTROL;

    let mut opts = OpenOptions::new();
    opts.custom_flags(FILE_FLAG_BACKUP_SEMANTICS);

    // READ_CONTROL is needed as the basic access right
    // if *nothing* else is specified
    // here, we only check for not read nor write
    // TODO should this also take the result of
    // oflags and fdflags conversion into account?
    if !read && !write {
        opts.access_mode(READ_CONTROL);
    } else {
        opts.read(read).write(write);
    }

    // convert open flags
    host_impl::open_options_from_oflags(&mut opts, oflags);
    // convert file descriptor flags
    host_impl::open_options_from_fdflags(&mut opts, fdflags);

    let path = concatenate(&dirfd, Path::new(&path))?;
    opts.open(path).map_err(errno_from_ioerror)
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
    unimplemented!("path_symlink")
}

pub(crate) fn path_unlink_file(dirfd: File, path: String) -> Result<()> {
    let path = concatenate(&dirfd, Path::new(&path))?;
    std::fs::remove_file(path).map_err(errno_from_ioerror)
}

pub(crate) fn path_remove_directory(dirfd: File, path: String) -> Result<()> {
    let path = concatenate(&dirfd, Path::new(&path))?;
    std::fs::remove_dir(path).map_err(errno_from_ioerror)
}
