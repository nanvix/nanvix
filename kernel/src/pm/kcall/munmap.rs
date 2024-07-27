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
    mm::VirtMemoryManager,
    pm::ProcessManager,
};
use ::kcall::{
    Capability,
    Error,
    ErrorCode,
    ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_munmap(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    pid: ProcessIdentifier,
    vaddr: PageAligned<VirtualAddress>,
) -> Result<(), Error> {
    pm.munmap(mm, pid, vaddr)
}

pub fn munmap(pm: &mut ProcessManager, mm: &mut VirtMemoryManager, args: &KcallArgs) -> i32 {
    // Check if the calling process has memory management capabilities.
    match ProcessManager::has_capability(args.pid, Capability::MemoryManagement) {
        Ok(true) => (),
        Ok(false) => {
            let reason: &str = "process does not have memory management capabilities";
            error!("mmap(): {}", reason);
            return ErrorCode::PermissionDenied.into_errno();
        },
        Err(e) => return e.code.into_errno(),
    }

    // Unpack kernel call arguments.
    let pid: ProcessIdentifier = ProcessIdentifier::from(args.arg0 as usize);
    let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(args.arg1 as usize) {
        Ok(vaddr) => vaddr,
        Err(e) => return e.code.into_errno(),
    };

    match do_munmap(pm, mm, pid, vaddr) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
