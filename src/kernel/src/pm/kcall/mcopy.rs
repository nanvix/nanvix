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
    mm::{
        KernelPage,
        VirtMemoryManager,
    },
    pm::ProcessManager,
};
use ::arch::mem;
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

fn do_mcopy(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    src_pid: ProcessIdentifier,
    src_vaddr: PageAligned<VirtualAddress>,
    dst_pid: ProcessIdentifier,
    dst_vaddr: PageAligned<VirtualAddress>,
) -> Result<(), Error> {
    // Allocate a kernel page to use a scratch memory.
    let kpage: KernelPage = mm.alloc_kpage(true)?;

    // Copy to kernel page.
    pm.vmcopy_from_user(
        src_pid,
        kpage.base().into_virtual_address().into_inner(),
        src_vaddr.into_inner(),
        mem::PAGE_SIZE,
    )?;

    // Copy from kernel page.
    pm.vmcopy_to_user(
        dst_pid,
        dst_vaddr.into_inner(),
        kpage.base().into_virtual_address().into_inner(),
        mem::PAGE_SIZE,
    )?;

    Ok(())
}

pub fn mcopy(pm: &mut ProcessManager, mm: &mut VirtMemoryManager, args: &KcallArgs) -> KcallResult {
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

    // Unpack kernel call arguments.
    let src_pid: ProcessIdentifier = ProcessIdentifier::from(args.arg0);
    let src_vaddr: PageAligned<VirtualAddress> =
        match PageAligned::from_raw_value(args.arg1 as usize) {
            Ok(vaddr) => vaddr,
            Err(e) => return KcallResult::Error(e.code.into()),
        };
    let dst_pid: ProcessIdentifier = ProcessIdentifier::from(args.arg2);
    let dst_vaddr: PageAligned<VirtualAddress> =
        match PageAligned::from_raw_value(args.arg3 as usize) {
            Ok(vaddr) => vaddr,
            Err(e) => return KcallResult::Error(e.code.into()),
        };

    match do_mcopy(pm, mm, src_pid, src_vaddr, dst_pid, dst_vaddr) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
