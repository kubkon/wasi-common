use crate::{winerror, Result};
use std::os::windows::prelude::RawHandle;
use winapi::shared::minwindef;
use winapi::um::{fileapi::GetFileType, winbase, winnt};

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
