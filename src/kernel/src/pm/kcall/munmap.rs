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
    mm::VirtMemoryManager,
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

fn do_munmap(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    pid: ProcessIdentifier,
    vaddr: PageAligned<VirtualAddress>,
) -> Result<(), Error> {
    pm.munmap(mm, pid, vaddr)
}

pub fn munmap(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    args: &KcallArgs,
) -> KcallResult {
    // Unpack kernel call arguments.
    let pid: ProcessIdentifier = match ProcessIdentifier::try_from(args.arg0) {
        Ok(pid) => pid,
        Err(error) => {
            error!("munmap(): {error:?}");
            return KcallResult::Error(error.code.into());
        },
    };
    let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(args.arg1 as usize) {
        Ok(vaddr) => vaddr,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    // Check if attempting to unmap memory from  a different process.
    if pid != args.pid {
        // Check if the calling process has memory management capabilities.
        match pm.has_capability(args.pid, Capability::MemoryManagement) {
            Ok(true) => (),
            Ok(false) => {
                let reason: &str = "process does not have memory management capabilities";
                error!("mmap(): {}", reason);
                return KcallResult::Error(ErrorCode::PermissionDenied.into());
            },
            Err(e) => return KcallResult::Error(e.code.into()),
        }
    }

    match do_munmap(pm, mm, pid, vaddr) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
