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
        MmioRegionInfo,
        VirtualAddress,
    },
    number::KcallNumber,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Helpers
//==================================================================================================

fn split_tag(tag: u64) -> (u32, u32) {
    let lower: u32 = tag as u32;
    let upper: u32 = (tag >> 32) as u32;
    (lower, upper)
}

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
// Allocate MMIO Region
//==================================================================================================

///
/// # Description
///
/// Requests the kernel to attach a memory-mapped I/O region identified by `tag` to the
/// calling process. The tag must match one registered via the hardware abstraction layer and must
/// be encoded as up to 16 hexadecimal digits packed into a 64-bit value (four bits per digit).
///
/// # Parameters
///
/// - `tag`: Encoded identifier of the MMIO region to allocate.
///
/// # Returns
///
/// `Ok(())` on success, or an error describing why the allocation failed.
///
pub fn mmio_alloc(tag: u64) -> Result<(), Error> {
    let (lower, upper): (u32, u32) = split_tag(tag);
    let result: i64 = kcall2!(KcallNumber::AllocMmio.into(), lower, upper);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to allocate mmio region"))
    }
}

//==================================================================================================
// Release MMIO Region
//==================================================================================================

///
/// # Description
///
/// Releases a previously allocated memory-mapped I/O region identified by `tag` from the calling
/// process.
///
/// # Parameters
///
/// - `tag`: Encoded identifier of the MMIO region to release.
///
/// # Returns
///
/// `Ok(())` on success, or an error describing why the release failed.
///
pub fn mmio_free(tag: u64) -> Result<(), Error> {
    let (lower, upper): (u32, u32) = split_tag(tag);
    let result: i64 = kcall2!(KcallNumber::FreeMmio.into(), lower, upper);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to free mmio region"))
    }
}

//==================================================================================================
// Query MMIO Region
//==================================================================================================

///
/// # Description
///
/// Queries metadata for the memory-mapped I/O region identified by `tag` and returns information
/// such as the mapped base address, size, and permissions.
///
/// # Parameters
///
/// - `tag`: Encoded identifier of the MMIO region to query.
///
/// # Returns
///
/// On success, returns an [`MmioRegionInfo`] structure populated by the kernel.
///
pub fn mmio_info(tag: u64) -> Result<MmioRegionInfo, Error> {
    let (lower, upper): (u32, u32) = split_tag(tag);
    let mut info: MmioRegionInfo = MmioRegionInfo::default();
    let buffer: *mut MmioRegionInfo = &mut info;

    let result: i64 = kcall3!(KcallNumber::MmioInfo.into(), lower, upper, buffer as usize as u32);

    if result == 0 {
        Ok(info)
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to query mmio region"))
    }
}
