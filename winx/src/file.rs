#![allow(non_camel_case_types)]
use crate::{winerror, Result};
use std::ffi::{OsStr, OsString};
use std::os::windows::prelude::{OsStrExt, OsStringExt, RawHandle};
use winapi::shared::minwindef::{self, DWORD};
use winapi::um::{fileapi, fileapi::GetFileType, winbase, winnt};

/// Maximum total path length for Unicode in Windows.
/// [Maximum path length limitation]: https://docs.microsoft.com/en-us/windows/desktop/FileIO/naming-a-file#maximum-path-length-limitation
pub const WIDE_MAX_PATH: DWORD = 0x7fff;

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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum CreationDisposition {
    NO_DISPOSITION = 0,
    /// Creates a new file, only if it does not already exist.
    /// If the specified file exists, the function fails and the last-error code is
    /// set to ERROR_FILE_EXISTS (80).
    ///
    /// If the specified file does not exist and is a valid path to a writable location,
    /// a new file is created.
    CREATE_NEW = fileapi::CREATE_NEW,
    /// Creates a new file, always.
    /// If the specified file exists and is writable, the function overwrites the file,
    /// the function succeeds, and last-error code is set to ERROR_ALREADY_EXISTS (183).
    ///
    /// If the specified file does not exist and is a valid path, a new file is created,
    /// the function succeeds, and the last-error code is set to zero.
    CREATE_ALWAYS = fileapi::CREATE_ALWAYS,
    /// Opens a file or device, only if it exists.
    /// If the specified file or device does not exist, the function fails and the
    /// last-error code is set to ERROR_FILE_NOT_FOUND (2).
    OPEN_EXISTING = fileapi::OPEN_EXISTING,
    /// Opens a file, always.
    /// If the specified file exists, the function succeeds and the last-error code is
    /// set to ERROR_ALREADY_EXISTS (183).
    ///
    /// If the specified file does not exist and is a valid path to a writable location,
    /// the function creates a file and the last-error code is set to zero.
    OPEN_ALWAYS = fileapi::OPEN_ALWAYS,
    /// Opens a file and truncates it so that its size is zero bytes, only if it exists.
    /// If the specified file does not exist, the function fails and the last-error code
    /// is set to ERROR_FILE_NOT_FOUND (2).
    ///
    /// The calling process must open the file with the GENERIC_WRITE bit set as part
    /// of the dwDesiredAccess parameter.
    TRUNCATE_EXISTING = fileapi::TRUNCATE_EXISTING,
}

impl CreationDisposition {
    pub fn from_u32(disp: u32) -> Self {
        use CreationDisposition::*;
        match disp {
            fileapi::CREATE_NEW => CREATE_NEW,
            fileapi::CREATE_ALWAYS => CREATE_ALWAYS,
            fileapi::OPEN_EXISTING => OPEN_EXISTING,
            fileapi::OPEN_ALWAYS => OPEN_ALWAYS,
            fileapi::TRUNCATE_EXISTING => TRUNCATE_EXISTING,
            _ => NO_DISPOSITION,
        }
    }
}

bitflags! {
    pub struct FlagsAndAttributes: minwindef::DWORD {
        const FILE_ATTRIBUTE_ARCHIVE = winnt::FILE_ATTRIBUTE_ARCHIVE;
        const FILE_ATTRIBUTE_ENCRYPTED = winnt::FILE_ATTRIBUTE_ENCRYPTED;
        const FILE_ATTRIBUTE_HIDDEN = winnt::FILE_ATTRIBUTE_HIDDEN;
        const FILE_ATTRIBUTE_NORMAL = winnt::FILE_ATTRIBUTE_NORMAL;
        const FILE_ATTRIBUTE_OFFLINE = winnt::FILE_ATTRIBUTE_OFFLINE;
        const FILE_ATTRIBUTE_READONLY = winnt::FILE_ATTRIBUTE_READONLY;
        const FILE_ATTRIBUTE_SYSTEM = winnt::FILE_ATTRIBUTE_SYSTEM;
        const FILE_ATTRIBUTE_TEMPORARY = winnt::FILE_ATTRIBUTE_TEMPORARY;
        const FILE_FLAG_BACKUP_SEMANTICS = winbase::FILE_FLAG_BACKUP_SEMANTICS;
        const FILE_FLAG_DELETE_ON_CLOSE = winbase::FILE_FLAG_DELETE_ON_CLOSE;
        const FILE_FLAG_NO_BUFFERING = winbase::FILE_FLAG_NO_BUFFERING;
        const FILE_FLAG_OPEN_NO_RECALL = winbase::FILE_FLAG_OPEN_NO_RECALL;
        const FILE_FLAG_OPEN_REPARSE_POINT = winbase::FILE_FLAG_OPEN_REPARSE_POINT;
        const FILE_FLAG_OVERLAPPED = winbase::FILE_FLAG_OVERLAPPED;
        const FILE_FLAG_POSIX_SEMANTICS = winbase::FILE_FLAG_POSIX_SEMANTICS;
        const FILE_FLAG_RANDOM_ACCESS = winbase::FILE_FLAG_RANDOM_ACCESS;
        const FILE_FLAG_SESSION_AWARE = winbase::FILE_FLAG_SESSION_AWARE;
        const FILE_FLAG_SEQUENTIAL_SCAN = winbase::FILE_FLAG_SEQUENTIAL_SCAN;
        const FILE_FLAG_WRITE_THROUGH = winbase::FILE_FLAG_WRITE_THROUGH;
    }
}

bitflags! {
    /// [Access mask]: https://docs.microsoft.com/en-us/windows/desktop/SecAuthZ/access-mask
    pub struct AccessRight: minwindef::DWORD {
        /// For a file object, the right to read the corresponding file data.
        /// For a directory object, the right to read the corresponding directory data.
        const READ = winnt::FILE_READ_DATA;
        /// For a file object, the right to write data to the file.
        /// For a directory object, the right to create a file in the directory.
        const WRITE = winnt::FILE_WRITE_DATA;
        /// For a file object, the right to append data to the file.
        /// (For local files, write operations will not overwrite existing data
        /// if this flag is specified without FILE_WRITE_DATA.)
        /// For a directory object, the right to create a subdirectory.
        /// For a named pipe, the right to create a pipe.
        const APPEND = winnt::FILE_APPEND_DATA;
        /// The right to read extended file attributes.
        const READ_EA = winnt::FILE_READ_EA;
        /// The right to write extended file attributes.
        const WRITE_EA = winnt::FILE_WRITE_EA;
        /// For a directory, the right to traverse the directory.
        /// By default, users are assigned the BYPASS_TRAVERSE_CHECKING privilege,
        /// which ignores the FILE_TRAVERSE access right.
        const TRAVERSE = winnt::FILE_TRAVERSE;
        /// For a directory, the right to delete a directory and all
        /// the files it contains, including read-only files.
        const DELETE_CHILD = winnt::FILE_DELETE_CHILD;
        /// The right to read file attributes.
        const READ_ATTRIBUTES = winnt::FILE_READ_ATTRIBUTES;
        /// The right to write file attributes.
        const WRITE_ATTRIBUTES = winnt::FILE_WRITE_ATTRIBUTES;
        /// The right to delete the object.
        const DELETE = winnt::DELETE;
        /// The right to read the information in the object's security descriptor,
        /// not including the information in the system access control list (SACL).
        const READ_CONTROL = winnt::READ_CONTROL;
        /// The right to use the object for synchronization. This enables a thread
        /// to wait until the object is in the signaled state. Some object types
        /// do not support this access right.
        const SYNCHRONIZE = winnt::SYNCHRONIZE;
        /// The right to modify the discretionary access control list (DACL) in
        /// the object's security descriptor.
        const WRITE_DAC = winnt::WRITE_DAC;
        /// The right to change the owner in the object's security descriptor.
        const WRITE_OWNER = winnt::WRITE_OWNER;
        /// It is used to indicate access to a system access control list (SACL).
        const ACCESS_SYSTEM_SECURITY = winnt::ACCESS_SYSTEM_SECURITY;
        /// Maximum allowed.
        const MAXIMUM_ALLOWED = winnt::MAXIMUM_ALLOWED;
        /// Reserved
        const RESERVED1 = 0x4000000;
        /// Reserved
        const RESERVED2 = 0x8000000;
        /// Provides all possible access rights.
        const GENERIC_ALL = winnt::GENERIC_ALL;
        /// Provides execute access.
        const GENERIC_EXECUTE = winnt::GENERIC_EXECUTE;
        /// Provides write access.
        const GENERIC_WRITE = winnt::GENERIC_WRITE;
        /// Provides read access.
        const GENERIC_READ = winnt::GENERIC_READ;
    }
}

pub fn get_file_access_rights(handle: RawHandle) -> Result<minwindef::DWORD> {
    use winapi::shared::minwindef::FALSE;
    use winapi::um::accctrl;
    use winapi::um::aclapi::GetSecurityInfo;
    use winapi::um::securitybaseapi::{GetAce, IsValidAcl};
    unsafe {
        let mut dacl = 0 as winnt::PACL;
        let mut sec_desc = 0 as winnt::PSECURITY_DESCRIPTOR;

        let err = winerror::WinError::from_u32(GetSecurityInfo(
            handle,
            accctrl::SE_FILE_OBJECT,
            winnt::DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut dacl,
            std::ptr::null_mut(),
            &mut sec_desc,
        ));

        if err != winerror::WinError::ERROR_SUCCESS {
            return Err(err);
        }

        if IsValidAcl(dacl) == FALSE {
            return Err(winerror::WinError::last());
        }

        // let count = (*dacl).AceCount;
        let mut ace = 0 as winnt::PVOID;

        if GetAce(dacl, 0, &mut ace) == FALSE {
            return Err(winerror::WinError::last());
        }

        // TODO: check for PACCESS_ALLOWED_ACE in Ace before accessing
        // let header = (*(ace as winnt::PACCESS_ALLOWED_ACE)).Header.AceType;
        Ok((*(ace as winnt::PACCESS_ALLOWED_ACE)).Mask)
    }
}

/// Converts OS string reference to Windows wide UTF-16 format.
pub fn str_to_wide<S: AsRef<OsStr>>(s: S) -> Vec<u16> {
    let mut win_unicode: Vec<u16> = s.as_ref().encode_wide().collect();
    win_unicode.push(0);
    win_unicode
}

/// Opens a `path` relative to a directory handle `dir_handle`, and returns a handle to the
/// newly opened file. The newly opened file will have the specified `AccessRight` `rights`.
///
/// If the `path` is absolute, then the directory handle `dir_handle` is ignored.
pub fn openat<S: AsRef<OsStr>>(
    dir_handle: RawHandle,
    path: S,
    rights: AccessRight,
    disposition: CreationDisposition,
    flags_attrs: FlagsAndAttributes,
) -> Result<RawHandle> {
    use std::path::PathBuf;
    use winapi::um::fileapi::{CreateFileW, GetFinalPathNameByHandleW};
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;

    // check if specified path is absolute
    let path = PathBuf::from(path.as_ref());
    let out_path = if path.is_absolute() {
        path
    } else {
        let mut raw_dir_path: Vec<u16> = Vec::with_capacity(WIDE_MAX_PATH as usize);
        raw_dir_path.resize(WIDE_MAX_PATH as usize, 0);

        let read_len = unsafe {
            GetFinalPathNameByHandleW(dir_handle, raw_dir_path.as_mut_ptr(), WIDE_MAX_PATH, 0)
        };

        if read_len == 0 {
            // failed to read
            return Err(winerror::WinError::last());
        }
        if read_len > WIDE_MAX_PATH {
            // path too long (practically probably impossible)
            return Err(winerror::WinError::ERROR_BAD_PATHNAME);
        }

        // concatenate paths
        raw_dir_path.resize(read_len as usize, 0);
        let mut out_path = PathBuf::from(OsString::from_wide(&raw_dir_path));
        out_path.push(path);
        out_path
    };

    let raw_out_path = str_to_wide(out_path);
    let handle = unsafe {
        CreateFileW(
            raw_out_path.as_ptr(),
            rights.bits(),
            0,
            std::ptr::null_mut(),
            disposition as minwindef::DWORD,
            flags_attrs.bits(),
            std::ptr::null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE {
        Err(winerror::WinError::last())
    } else {
        Ok(handle)
    }
}
