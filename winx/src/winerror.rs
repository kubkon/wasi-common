#![allow(non_camel_case_types)]
use winapi::shared::winerror;
use winapi::um::errhandlingapi::GetLastError;

macro_rules! win_error_expand {
    {
        $(
            #[doc=$doc:literal]
            $error:ident,
        )*
    } => {
        /// Wraps WINAPI error code as enum.
        #[derive(Debug, Clone, Copy, Eq, PartialEq)]
        #[repr(u32)]
        pub enum WinError {
            /// Unknown error occurred.
            UnknownError = std::u32::MAX,
            $(
                #[doc=$doc]
                $error = winerror::$error,
            )*
        }

        fn desc(err: WinError) -> &'static str {
            use WinError::*;
            match err {
                UnknownError => r" Unknown error occurred.",
                $($error => $doc,)*
            }
        }

        fn from_u32(err: u32) -> WinError {
            use WinError::*;
            match err {
                $(winerror::$error => $error,)*
                _ => UnknownError,
            }
        }
    }
}

win_error_expand! {
    /// The operation completed successfully.
    ERROR_SUCCESS,
    /// Incorrect function.
    ERROR_INVALID_FUNCTION,
    /// The system cannot find the file specified.
    ERROR_FILE_NOT_FOUND,
    /// The system cannot find the path specified.
    ERROR_PATH_NOT_FOUND,
    /// The system cannot open the file.
    ERROR_TOO_MANY_OPEN_FILES,
    /// Access is denied.
    ERROR_ACCESS_DENIED,
    /// The handle is invalid.
    ERROR_INVALID_HANDLE,
    /// The storage control blocks were destroyed.
    ERROR_ARENA_TRASHED,
    /// Not enough storage is available to process this command.
    ERROR_NOT_ENOUGH_MEMORY,
    /// The storage control block address is invalid.
    ERROR_INVALID_BLOCK,
    /// The environment is incorrect.
    ERROR_BAD_ENVIRONMENT,
    /// An attempt was made to load a program with an incorrect format.
    ERROR_BAD_FORMAT,
    /// The access code is invalid.
    ERROR_INVALID_ACCESS,
    /// The data is invalid.
    ERROR_INVALID_DATA,
    /// Not enough storage is available to complete this operation.
    ERROR_OUTOFMEMORY,
    /// The system cannot find the drive specified.
    ERROR_INVALID_DRIVE,
    /// The directory cannot be removed.
    ERROR_CURRENT_DIRECTORY,
    /// The system cannot move the file to a different disk drive.
    ERROR_NOT_SAME_DEVICE,
    /// There are no more files.
    ERROR_NO_MORE_FILES,
    /// The media is write-protected.
    ERROR_WRITE_PROTECT,
    /// The system cannot find the device specified.
    ERROR_BAD_UNIT,
    /// The device is not ready.
    ERROR_NOT_READY,
    /// The device does not recognize the command.
    ERROR_BAD_COMMAND,
    /// Data error (cyclic redundancy check).
    ERROR_CRC,
    /// The program issued a command but the command length is incorrect.
    ERROR_BAD_LENGTH,
    /// The drive cannot locate a specific area or track on the disk.
    ERROR_SEEK,
    /// The specified disk cannot be accessed.
    ERROR_NOT_DOS_DISK,
    /// The drive cannot find the sector requested.
    ERROR_SECTOR_NOT_FOUND,
    /// The printer is out of paper.
    ERROR_OUT_OF_PAPER,
    /// The system cannot write to the specified device.
    ERROR_WRITE_FAULT,
    /// The system cannot read from the specified device.
    ERROR_READ_FAULT,
    /// A device attached to the system is not functioning.
    ERROR_GEN_FAILURE,
    /// The process cannot access the file because it is being used by another process.
    ERROR_SHARING_VIOLATION,
    /// The process cannot access the file because another process has locked a portion of the file.
    ERROR_LOCK_VIOLATION,
    /// The wrong disk is in the drive. Insert %2 (Volume Serial Number: %3) into drive %1.
    ERROR_WRONG_DISK,
    /// Too many files opened for sharing.
    ERROR_SHARING_BUFFER_EXCEEDED,
    /// Reached the end of the file.
    ERROR_HANDLE_EOF,
    /// The disk is full.
    ERROR_HANDLE_DISK_FULL,
    /// The request is not supported.
    ERROR_NOT_SUPPORTED,
    /// Windows cannot find the network path.
    ERROR_REM_NOT_LIST,
    /// You were not connected because a duplicate name exists on the network.
    ERROR_DUP_NAME,
    /// The network path was not found.
    ERROR_BAD_NETPATH,
    /// The network is busy.
    ERROR_NETWORK_BUSY,
    /// The specified network resource or device is no longer available.
    ERROR_DEV_NOT_EXIST,
    /// The network BIOS command limit has been reached.
    ERROR_TOO_MANY_CMDS,
    /// A network adapter hardware error occurred.
    ERROR_ADAP_HDW_ERR,
    /// The specified server cannot perform the requested operation.
    ERROR_BAD_NET_RESP,
    /// An unexpected network error occurred.
    ERROR_UNEXP_NET_ERR,
    /// The remote adapter is not compatible.
    ERROR_BAD_REM_ADAP,
    /// The print queue is full.
    ERROR_PRINTQ_FULL,
    /// Space to store the file waiting to be printed is not available on the server.
    ERROR_NO_SPOOL_SPACE,
    /// Your file waiting to be printed was deleted.
    ERROR_PRINT_CANCELLED,
    /// The specified network name is no longer available.
    ERROR_NETNAME_DELETED,
    /// Network access is denied.
    ERROR_NETWORK_ACCESS_DENIED,
    /// The network resource type is not correct.
    ERROR_BAD_DEV_TYPE,
    /// The network name cannot be found.
    ERROR_BAD_NET_NAME,
    /// The name limit for the local computer network adapter card was exceeded.
    ERROR_TOO_MANY_NAMES,
    /// The network BIOS session limit was exceeded.
    ERROR_TOO_MANY_SESS,
    /// The remote server has been paused or is in the process of being started.
    ERROR_SHARING_PAUSED,
    /// No more connections can be made to this remote computer at this time because the computer has accepted the maximum number of connections.
    ERROR_REQ_NOT_ACCEP,
    /// The specified printer or disk device has been paused.
    ERROR_REDIR_PAUSED,
    /// The file exists.
    ERROR_FILE_EXISTS,
    /// The directory or file cannot be created.
    ERROR_CANNOT_MAKE,
    /// Fail on INT 24.
    ERROR_FAIL_I24,
    /// Storage to process this request is not available.
    ERROR_OUT_OF_STRUCTURES,
    /// The local device name is already in use.
    ERROR_ALREADY_ASSIGNED,
    /// The specified network password is not correct.
    ERROR_INVALID_PASSWORD,
    /// The parameter is incorrect.
    ERROR_INVALID_PARAMETER,
    /// A write fault occurred on the network.
    ERROR_NET_WRITE_FAULT,
    /// The system cannot start another process at this time.
    ERROR_NO_PROC_SLOTS,
    /// Cannot create another system semaphore.
    ERROR_TOO_MANY_SEMAPHORES,
    /// Cannot create another system semaphore.
    ERROR_EXCL_SEM_ALREADY_OWNED,
    /// The semaphore is set and cannot be closed.
    ERROR_SEM_IS_SET,
    /// The semaphore cannot be set again.
    ERROR_TOO_MANY_SEM_REQUESTS,
    /// Cannot request exclusive semaphores at interrupt time.
    ERROR_INVALID_AT_INTERRUPT_TIME,
    /// The previous ownership of this semaphore has ended.
    ERROR_SEM_OWNER_DIED,
    /// Insert the disk for drive %1.
    ERROR_SEM_USER_LIMIT,
    /// The program stopped because an alternate disk was not inserted.
    ERROR_DISK_CHANGE,
    /// The disk is in use or locked by another process.
    ERROR_DRIVE_LOCKED,
    /// The pipe has been ended.
    ERROR_BROKEN_PIPE,
    /// The system cannot open the device or file specified.
    ERROR_OPEN_FAILED,
    /// The file name is too long.
    ERROR_BUFFER_OVERFLOW,
    /// There is not enough space on the disk.
    ERROR_DISK_FULL,
    /// No more internal file identifiers are available.
    ERROR_NO_MORE_SEARCH_HANDLES,
    /// The target internal file identifier is incorrect.
    ERROR_INVALID_TARGET_HANDLE,
    /// The Input Output Control (IOCTL) call made by the application program is not correct.
    ERROR_INVALID_CATEGORY,
    /// The verify-on-write switch parameter value is not correct.
    ERROR_INVALID_VERIFY_SWITCH,
    /// The system does not support the command requested.
    ERROR_BAD_DRIVER_LEVEL,
    /// This function is not supported on this system.
    ERROR_CALL_NOT_IMPLEMENTED,
    /// The semaphore time-out period has expired.
    ERROR_SEM_TIMEOUT,
    /// The data area passed to a system call is too small.
    ERROR_INSUFFICIENT_BUFFER,
    /// The file name, directory name, or volume label syntax is incorrect.
    ERROR_INVALID_NAME,
    /// The system call level is not correct.
    ERROR_INVALID_LEVEL,
    /// The disk has no volume label.
    ERROR_NO_VOLUME_LABEL,
    /// The specified module could not be found.
    ERROR_MOD_NOT_FOUND,
    /// The specified procedure could not be found.
    ERROR_PROC_NOT_FOUND,
    /// There are no child processes to wait for.
    ERROR_WAIT_NO_CHILDREN,
    /// The %1 application cannot be run in Win32 mode.
    ERROR_CHILD_NOT_COMPLETE,
    /// Attempt to use a file handle to an open disk partition for an operation other than raw disk I/O.
    ERROR_DIRECT_ACCESS_HANDLE,
    /// An attempt was made to move the file pointer before the beginning of the file.
    ERROR_NEGATIVE_SEEK,
    /// The file pointer cannot be set on the specified device or file.
    ERROR_SEEK_ON_DEVICE,
    /// A JOIN or SUBST command cannot be used for a drive that contains previously joined drives.
    ERROR_IS_JOIN_TARGET,
    /// An attempt was made to use a JOIN or SUBST command on a drive that has already been joined.
    ERROR_IS_JOINED,
    /// An attempt was made to use a JOIN or SUBST command on a drive that has already been substituted.
    ERROR_IS_SUBSTED,
    /// The system tried to delete the JOIN of a drive that is not joined.
    ERROR_NOT_JOINED,
    /// The system tried to delete the substitution of a drive that is not substituted.
    ERROR_NOT_SUBSTED,
    /// The system tried to join a drive to a directory on a joined drive.
    ERROR_JOIN_TO_JOIN,
    /// The system tried to substitute a drive to a directory on a substituted drive.
    ERROR_SUBST_TO_SUBST,
    /// The system tried to join a drive to a directory on a substituted drive.
    ERROR_JOIN_TO_SUBST,
    /// The system tried to SUBST a drive to a directory on a joined drive.
    ERROR_SUBST_TO_JOIN,
    /// The system cannot perform a JOIN or SUBST at this time.
    ERROR_BUSY_DRIVE,
    /// The system cannot join or substitute a drive to or for a directory on the same drive.
    ERROR_SAME_DRIVE,
    /// The directory is not a subdirectory of the root directory.
    ERROR_DIR_NOT_ROOT,
    /// The directory is not empty.
    ERROR_DIR_NOT_EMPTY,
    /// The path specified is being used in a substitute.
    ERROR_IS_SUBST_PATH,
    /// Not enough resources are available to process this command.
    ERROR_IS_JOIN_PATH,
    /// The path specified cannot be used at this time.
    ERROR_PATH_BUSY,
    /// An attempt was made to join or substitute a drive for which a directory on the drive is the target of a previous substitute.
    ERROR_IS_SUBST_TARGET,
    /// System trace information was not specified in your CONFIG.SYS file, or tracing is disallowed.
    ERROR_SYSTEM_TRACE,
    /// The number of specified semaphore events for DosMuxSemWait is not correct.
    ERROR_INVALID_EVENT_COUNT,
    /// DosMuxSemWait did not execute; too many semaphores are already set.
    ERROR_TOO_MANY_MUXWAITERS,
    /// The DosMuxSemWait list is not correct.
    ERROR_INVALID_LIST_FORMAT,
    /// The volume label you entered exceeds the label character limit of the destination file system.
    ERROR_LABEL_TOO_LONG,
    /// Cannot create another thread.
    ERROR_TOO_MANY_TCBS,
    /// The recipient process has refused the signal.
    ERROR_SIGNAL_REFUSED,
    /// The segment is already discarded and cannot be locked.
    ERROR_DISCARDED,
    /// The segment is already unlocked.
    ERROR_NOT_LOCKED,
    /// The address for the thread ID is not correct.
    ERROR_BAD_THREADID_ADDR,
    /// One or more arguments are not correct.
    ERROR_BAD_ARGUMENTS,
    /// The specified path is invalid.
    ERROR_BAD_PATHNAME,
}

impl WinError {
    /// Returns the last error as WinError.
    pub fn last() -> Self {
        Self::from_u32(unsafe { GetLastError() })
    }

    /// Constructs WinError from error code.
    pub fn from_u32(err: u32) -> Self {
        from_u32(err)
    }

    /// Returns error's description string. This description matches
    /// the docs for the error.
    pub fn desc(self) -> &'static str {
        desc(self)
    }
}

impl std::error::Error for WinError {
    fn description(&self) -> &str {
        self.desc()
    }
}

impl std::fmt::Display for WinError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}: {}", self, self.desc())
    }
}

impl From<WinError> for std::io::Error {
    fn from(err: WinError) -> Self {
        Self::from_raw_os_error(err as i32)
    }
}
