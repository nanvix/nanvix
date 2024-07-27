// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    mm::AccessPermission,
    Error,
    ErrorCode,
    KcallNumber,
    ProcessIdentifier,
};

//==================================================================================================
// Map Memory Page
//==================================================================================================

pub fn mmap(pid: ProcessIdentifier, vaddr: usize, access: AccessPermission) -> Result<(), Error> {
    let result: i32 = unsafe {
        crate::kcall3(KcallNumber::MemoryMap.into(), pid.into(), vaddr as u32, access.into())
    };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to mmap()"))
    }
}

//==================================================================================================
// Unmap Memory Page
//==================================================================================================

pub fn munmap(pid: ProcessIdentifier, vaddr: usize) -> Result<(), Error> {
    let result: i32 =
        unsafe { crate::kcall2(KcallNumber::MemoryUnmap.into(), pid.into(), vaddr as u32) };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to munmap()"))
    }
}
