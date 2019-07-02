#![allow(non_camel_case_types)]
use crate::{winerror, Result};
use std::os::windows::prelude::RawHandle;
use winapi::shared::minwindef::FALSE;

pub fn close(handle: RawHandle) -> Result<()> {
    use winapi::um::handleapi::CloseHandle;
    if unsafe { CloseHandle(handle) } == FALSE {
        Err(winerror::WinError::last())
    } else {
        Ok(())
    }
}