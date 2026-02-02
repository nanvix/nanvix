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
    pm::ProcessManager,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        Capability,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Detaches a memory-mapped I/O region identified by `tag` from a process.
///
/// # Parameters
///
/// - `pm`: Reference to the process manager.
/// - `pid`: Identifier of the calling process.
/// - `tag`: Tag that uniquely identifies the MMIO region.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
fn do_mmio_free(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    tag: MmioTag,
) -> Result<(), Error> {
    trace!("pid={:?}, tag={:?}", pid, tag);

    // Check if process does not have I/O management capabilities.
    if !pm.has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have I/O management capabilities";
        error!("{reason}");
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    // Detach I/O memory region from the process.
    pm.mmio_free(pid, tag)?;

    Ok(())
}

///
/// # Description
///
/// Kernel call handler for releasing an MMIO region.
///
/// # Parameters
///
/// - `pm`: Reference to the process manager.
/// - `args`: Kernel call arguments containing the encoded tag.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn mmio_free(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Parse arguments.
    let tag_value: u64 = ((args.arg1 as u64) << 32) | args.arg0 as u64;
    let tag: MmioTag = MmioTag::from_u64(tag_value);

    match do_mmio_free(pm, args.pid, tag) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
