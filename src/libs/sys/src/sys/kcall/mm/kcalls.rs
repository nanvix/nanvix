// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    kcall2,
    kcall3,
    mm::{
        AccessPermission,
        Address,
        VirtualAddress,
    },
    number::KcallNumber,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Map Memory Page
//==================================================================================================

pub fn mmap(
    pid: ProcessIdentifier,
    vaddr: VirtualAddress,
    access: AccessPermission,
) -> Result<(), Error> {
    let result: i64 = kcall3!(
        KcallNumber::MemoryMap.into(),
        pid.try_into()?,
        vaddr.into_raw_value() as u32,
        access.into()
    );

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to mmap()"))
    }
}

//==================================================================================================
// Unmap Memory Page
//==================================================================================================

pub fn munmap(pid: ProcessIdentifier, vaddr: VirtualAddress) -> Result<(), Error> {
    let result: i64 =
        kcall2!(KcallNumber::MemoryUnmap.into(), pid.try_into()?, vaddr.into_raw_value() as u32);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to munmap()"))
    }
}

//==================================================================================================
// Change Memory Protection
//==================================================================================================

pub fn mprotect(
    pid: ProcessIdentifier,
    vaddr: VirtualAddress,
    access: AccessPermission,
) -> Result<(), Error> {
    let result: i64 = kcall3!(
        KcallNumber::MemoryCtrl.into(),
        pid.try_into()?,
        vaddr.into_raw_value() as u32,
        access.into()
    );

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to mprotect()"))
    }
}
