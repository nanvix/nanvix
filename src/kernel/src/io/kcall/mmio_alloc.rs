// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        io::{
            IoMemoryRegion,
            MmioTag,
        },
        Hal,
    },
    kcall::KcallResult,
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

fn do_mmio_alloc(
    hal: &mut Hal,
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    tag: MmioTag,
) -> Result<(), Error> {
    trace!("pid={:?}, tag={:?}", pid, tag);

    // Check if process does not have I/O management capabilities.
    if !pm.has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have I/O management capabilities";
        error!("{}", reason);
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    // Attempt to allocate I/O memory region.
    let region: IoMemoryRegion = hal.ioaddresses().allocate(tag)?;

    // Attached I/O memory region to the process.
    pm.mmio_alloc(pid, region)?;

    Ok(())
}

///
/// # Description
///
/// Kernel call handler for allocating an MMIO region.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: Low 32 bits of the encoded MMIO tag.
/// - `arg1`: High 32 bits of the encoded MMIO tag.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn mmio_alloc(pid: ProcessIdentifier, arg0: u32, arg1: u32) -> KcallResult {
    // SAFETY: the hardware abstraction layer is initialized and access is synchronized.
    let hal: &mut Hal = unsafe { Hal::get_mut() };
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // Parse arguments.
    let tag_value: u64 = ((arg1 as u64) << 32) | arg0 as u64;
    let tag: MmioTag = MmioTag::from_u64(tag_value);

    match do_mmio_alloc(hal, pm, pid, tag) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
