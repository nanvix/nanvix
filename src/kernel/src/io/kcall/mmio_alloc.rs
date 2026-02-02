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
    let region: IoMemoryRegion = hal.ioaddresses.allocate(tag)?;

    // Attached I/O memory region to the process.
    pm.mmio_alloc(pid, region)?;

    Ok(())
}

pub fn mmio_alloc(hal: &mut Hal, pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    let tag_value: u64 = ((args.arg1 as u64) << 32) | args.arg0 as u64;
    let tag: MmioTag = MmioTag::from_u64(tag_value);

    match do_mmio_alloc(hal, pm, args.pid, tag) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
