#![allow(non_camel_case_types)]
use winapi::shared::winerror;
use winapi::um::errhandlingapi::GetLastError;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum WinError {
    UnknownError = std::u32::MAX,
    ERROR_SUCCESS = winerror::ERROR_SUCCESS,
    ERROR_INVALID_FUNCTION = winerror::ERROR_INVALID_FUNCTION,
    ERROR_FILE_NOT_FOUND = winerror::ERROR_FILE_NOT_FOUND,
    ERROR_PATH_NOT_FOUND = winerror::ERROR_PATH_NOT_FOUND,
    ERROR_TOO_MANY_OPEN_FILES = winerror::ERROR_TOO_MANY_OPEN_FILES,
    ERROR_ACCESS_DENIED = winerror::ERROR_ACCESS_DENIED,
    ERROR_INVALID_HANDLE = winerror::ERROR_INVALID_HANDLE,
    ERROR_ARENA_TRASHED = winerror::ERROR_ARENA_TRASHED,
    ERROR_NOT_ENOUGH_MEMORY = winerror::ERROR_NOT_ENOUGH_MEMORY,
    ERROR_INVALID_BLOCK = winerror::ERROR_INVALID_BLOCK,
    ERROR_BAD_ENVIRONMENT = winerror::ERROR_BAD_ENVIRONMENT,
    ERROR_BAD_FORMAT = winerror::ERROR_BAD_FORMAT,
    ERROR_INVALID_ACCESS = winerror::ERROR_INVALID_ACCESS,
    ERROR_INVALID_DATA = winerror::ERROR_INVALID_DATA,
    ERROR_OUTOFMEMORY = winerror::ERROR_OUTOFMEMORY,
}

impl WinError {
    pub fn last() -> Self {
        use WinError::*;
        match unsafe { GetLastError() } {
            winerror::ERROR_SUCCESS => ERROR_SUCCESS,
            winerror::ERROR_INVALID_FUNCTION => ERROR_INVALID_FUNCTION,
            _ => UnknownError,
        }
    }

    pub fn desc(self) -> &'static str {
        use WinError::*;
        match self {
            UnknownError => "Unknown error",
            ERROR_SUCCESS => "The operation completed successfully",
            ERROR_INVALID_FUNCTION => "Incorrect function",
            ERROR_FILE_NOT_FOUND => "The system cannot find the file specified",
            ERROR_PATH_NOT_FOUND => "The system cannot find the path specified",
            ERROR_TOO_MANY_OPEN_FILES => "The system cannot open the file",
            ERROR_ACCESS_DENIED => "Access is denied",
            ERROR_INVALID_HANDLE => "The handle is invalid",
            ERROR_ARENA_TRASHED => "The storage control blocks were destroyed",
            ERROR_NOT_ENOUGH_MEMORY => "Not enough storage is available to process this command",
            ERROR_INVALID_BLOCK => "The storage control block address is invalid",
            ERROR_BAD_ENVIRONMENT => "The environment is incorrect",
            ERROR_BAD_FORMAT => "An attempt was made to load a program with an incorrect format",
            ERROR_INVALID_ACCESS => "The access code is invalid",
            ERROR_INVALID_DATA => "The data is invalid",
            ERROR_OUTOFMEMORY => "Not enough storage is available to complete this operation",
        }
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
