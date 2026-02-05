// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::io::MmioTag,
    kcall::{
        KcallArgs,
        KcallResult,
    },
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
/// - `pm`: Reference to the process manager.
/// - `args`: Kernel call arguments containing the encoded tag and a pointer to the output buffer.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn mmio_info(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    let tag_value: u64 = ((args.arg1 as u64) << 32) | args.arg0 as u64;
    let tag: MmioTag = MmioTag::from_u64(tag_value);
    let buffer_ptr: *mut MmioRegionInfo = args.arg2 as usize as *mut MmioRegionInfo;

    if buffer_ptr.is_null() {
        return KcallResult::Error(ErrorCode::BadAddress.into());
    }

    match do_mmio_info(pm, args.pid, tag) {
        Ok(info) => match pm::copy_to_user(pm, args.pid, buffer_ptr, &info) {
            Ok(_) => KcallResult::ok(),
            Err(e) => KcallResult::Error(e.code.into()),
        },
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
