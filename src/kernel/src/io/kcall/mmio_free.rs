// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        io::MmioTag,
        mem::{
            Address,
            PageAligned,
            VirtualAddress,
        },
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

fn do_mmio_free(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    tag: MmioTag,
    addr: PageAligned<VirtualAddress>,
) -> Result<(), Error> {
    trace!("pid={:?}, tag={:?}, addr={:?}", pid, tag, addr.into_inner());

    // Check if process does not have I/O management capabilities.
    if !pm.has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have I/O management capabilities";
        error!("{reason}");
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    // Detached I/O memory region from the process.
    pm.mmio_free(pid, tag, addr)?;

    Ok(())
}

pub fn mmio_free(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Parse arguments.
    let tag_value: u64 = ((args.arg1 as u64) << 32) | args.arg0 as u64;
    let tag: MmioTag = MmioTag::from_u64(tag_value);
    let addr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(args.arg2 as usize) {
        Ok(base) => base,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    match do_mmio_free(pm, args.pid, tag, addr) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
