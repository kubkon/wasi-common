use crate::{winerror, Result};
use std::os::windows::prelude::RawHandle;
use winapi::shared::minwindef;
use winapi::um::{fileapi::GetFileType, winbase};

#[derive(Debug, Copy, Clone)]
pub struct FileType(minwindef::DWORD);

// possible types are:
// * FILE_TYPE_CHAR
// * FILE_TYPE_DISK
// * FILE_TYPE_PIPE
// * FILE_TYPE_REMOTE
// * FILE_TYPE_UNKNOWN
//
// FILE_TYPE_REMOTE is unused
// https://technet.microsoft.com/en-us/evalcenter/aa364960(v=vs.100)
impl FileType {
    /// Returns true if character device such as LPT device or console
    pub fn is_char(&self) -> bool {
        self.0 == winbase::FILE_TYPE_CHAR
    }

    /// Returns true if disk device such as file or dir
    pub fn is_disk(&self) -> bool {
        self.0 == winbase::FILE_TYPE_DISK
    }

    /// Returns true if pipe device such as socket, named pipe or anonymous pipe
    pub fn is_pipe(&self) -> bool {
        self.0 == winbase::FILE_TYPE_PIPE
    }

    /// Returns true if unknown device
    pub fn is_unknown(&self) -> bool {
        self.0 == winbase::FILE_TYPE_UNKNOWN
    }
}

pub fn get_file_type(handle: RawHandle) -> Result<FileType> {
    let file_type = unsafe { FileType(GetFileType(handle)) };
    let err = winerror::WinError::last();
    if file_type.is_unknown() && err != winerror::WinError::ERROR_SUCCESS {
        Err(err)
    } else {
        Ok(file_type)
    }
}
