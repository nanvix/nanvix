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
    kcall::KcallArgs,
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

fn do_iodetach(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    addr: PageAligned<VirtualAddress>,
) -> Result<(), Error> {
    trace!("iodetach(pid={:?}, addr={:?})", pid, addr.into_inner());

    // Check if process does not have I/O management capabilities.
    if !ProcessManager::has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have I/O management capabilities";
        error!("mmio_free(): {}", reason);
        return Err(Error::new(ErrorCode::PermissionDenied, &reason));
    }

    // Detached I/O memory region from the process.
    pm.detach_mmio(pid, addr)?;

    Ok(())
}

pub fn iodetach(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    // Parse arguments.
    let addr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(args.arg0 as usize) {
        Ok(base) => base,
        Err(e) => return e.code.into_errno(),
    };

    match do_iodetach(pm, args.pid, addr) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
