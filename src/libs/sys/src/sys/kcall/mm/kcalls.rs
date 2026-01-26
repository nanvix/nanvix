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

//==================================================================================================
// Allocate Memory-Mapped I/O Region
//==================================================================================================

///
/// # Description
///
/// Allocates a memory-mapped I/O region identified by the given tag.
///
/// # Parameters
///
/// - `tag`: A 64-bit tag that uniquely identifies the MMIO region (e.g., 8-byte ASCII name).
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn mmio_alloc(tag: u64) -> Result<(), Error> {
    let tag_lo: u32 = tag as u32;
    let tag_hi: u32 = (tag >> 32) as u32;
    let result: i64 = kcall2!(KcallNumber::AllocMmio.into(), tag_lo, tag_hi);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to mmio_alloc()"))
    }
}

//==================================================================================================
// Free Memory-Mapped I/O Region
//==================================================================================================

///
/// # Description
///
/// Frees a memory-mapped I/O region identified by the given tag and address.
///
/// # Parameters
///
/// - `tag`: A 64-bit tag that uniquely identifies the MMIO region.
/// - `vaddr`: Virtual address of the memory-mapped I/O region.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn mmio_free(tag: u64, vaddr: VirtualAddress) -> Result<(), Error> {
    let tag_lo: u32 = tag as u32;
    let tag_hi: u32 = (tag >> 32) as u32;
    let result: i64 =
        kcall3!(KcallNumber::FreeMmio.into(), tag_lo, tag_hi, vaddr.into_raw_value() as u32);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to mmio_free()"))
    }
}
