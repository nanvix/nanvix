// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        io::IoMemoryRegion,
        mem::{
            Address,
            PageAligned,
            VirtualAddress,
        },
        Hal,
    },
    kcall::KcallArgs,
    pm::ProcessManager,
};
use ::kcall::Error;
use kcall::{
    ErrorCode,
    ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_ioattach(
    hal: &mut Hal,
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    addr: PageAligned<VirtualAddress>,
) -> Result<(), Error> {
    trace!("ioattach(pid={:?}, addr={:?})", pid, addr.into_inner());

    // Check if process does not have I/O management capabilities.
    if !ProcessManager::has_capability(pid, kcall::Capability::IoManagement)? {
        let reason: &'static str = "process does not have I/O management capabilities";
        error!("mmio_alloc(): {}", reason);
        return Err(Error::new(ErrorCode::PermissionDenied, &reason));
    }

    // Attempt to allocate I/O memory region.
    let region: IoMemoryRegion = hal.ioaddresses.allocate(addr.into_inner())?;

    // Attached I/O memory region to the process.
    pm.attach_mmio(pid, region)?;

    Ok(())
}

pub fn ioattach(hal: &mut Hal, pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    // Parse arguments.
    let addr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(args.arg0 as usize) {
        Ok(base) => base,
        Err(e) => return e.code.into_errno(),
    };

    match do_ioattach(hal, pm, args.pid, addr) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
