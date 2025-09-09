// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        Address,
        PageAligned,
        VirtualAddress,
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
    addr: PageAligned<VirtualAddress>,
) -> Result<(), Error> {
    trace!("pid={:?}, addr={:?}", pid, addr.into_inner());

    // Check if process does not have I/O management capabilities.
    if !pm.has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have I/O management capabilities";
        error!("{reason}");
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    // Detached I/O memory region from the process.
    pm.mmio_free(pid, addr)?;

    Ok(())
}

pub fn mmio_free(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Parse arguments.
    let addr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(args.arg0 as usize) {
        Ok(base) => base,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    match do_mmio_free(pm, args.pid, addr) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
