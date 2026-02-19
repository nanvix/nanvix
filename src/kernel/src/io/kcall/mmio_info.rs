// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::io::MmioTag,
    kcall::KcallResult,
    pm::{
        self,
        ProcessManager,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::MmioRegionInfo,
    pm::{
        Capability,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Helpers
//==================================================================================================

///
/// # Description
///
/// Retrieves metadata for a memory-mapped I/O region attached to a process.
///
/// # Parameters
///
/// - `pm`: Reference to the process manager.
/// - `pid`: Identifier of the calling process.
/// - `tag`: Tag that uniquely identifies the MMIO region.
///
/// # Returns
///
/// Upon success, a populated [`MmioRegionInfo`] is returned. Upon failure, an error is returned
/// instead.
///
fn do_mmio_info(
    pm: &ProcessManager,
    pid: ProcessIdentifier,
    tag: MmioTag,
) -> Result<MmioRegionInfo, Error> {
    trace!("pid={pid:?}, tag={tag:?}");

    if !pm.has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have I/O management capabilities";
        error!("{reason}");
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    let (base, size, perm) = pm.mmio_info(pid, tag)?;
    MmioRegionInfo::new(base.into_inner(), size, perm)
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for querying metadata of an MMIO region.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: Low 32 bits of the encoded MMIO tag.
/// - `arg1`: High 32 bits of the encoded MMIO tag.
/// - `arg2`: Pointer to the output buffer for [`MmioRegionInfo`].
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn mmio_info(pid: ProcessIdentifier, arg0: u32, arg1: u32, arg2: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    let tag_value: u64 = ((arg1 as u64) << 32) | arg0 as u64;
    let tag: MmioTag = MmioTag::from_u64(tag_value);
    let buffer_ptr: *mut MmioRegionInfo = arg2 as usize as *mut MmioRegionInfo;

    if buffer_ptr.is_null() {
        return KcallResult::Error(ErrorCode::BadAddress.into());
    }

    match do_mmio_info(pm, pid, tag) {
        Ok(info) => match pm::copy_to_user(pm, pid, buffer_ptr, &info) {
            Ok(_) => KcallResult::ok(),
            Err(e) => KcallResult::Error(e.code.into()),
        },
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
