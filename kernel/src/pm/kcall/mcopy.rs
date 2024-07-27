// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
    hal::mem::{
        Address,
        PageAligned,
        VirtualAddress,
    },
    kcall::KcallArgs,
    mm::{
        KernelPage,
        VirtMemoryManager,
    },
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

fn do_mcopy(
    mm: &mut VirtMemoryManager,
    src_pid: ProcessIdentifier,
    src_vaddr: PageAligned<VirtualAddress>,
    dst_pid: ProcessIdentifier,
    dst_vaddr: PageAligned<VirtualAddress>,
) -> Result<(), Error> {
    // Allocate a kernel page to use a scratch memory.
    let kpage: KernelPage = mm.alloc_kpage(true)?;

    // Copy to kernel page.
    ProcessManager::vmcopy_from_user(
        src_pid,
        kpage.base().into_virtual_address().into_inner(),
        src_vaddr.into_inner(),
        mem::PAGE_SIZE,
    )?;

    // Copy from kernel page.
    ProcessManager::vmcopy_to_user(
        dst_pid,
        dst_vaddr.into_inner(),
        kpage.base().into_virtual_address().into_inner(),
        mem::PAGE_SIZE,
    )?;

    Ok(())
}

pub fn mcopy(mm: &mut VirtMemoryManager, args: &KcallArgs) -> i32 {
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
    let src_pid: ProcessIdentifier = ProcessIdentifier::from(args.arg0 as usize);
    let src_vaddr: PageAligned<VirtualAddress> =
        match PageAligned::from_raw_value(args.arg1 as usize) {
            Ok(vaddr) => vaddr,
            Err(e) => return e.code.into_errno(),
        };
    let dst_pid: ProcessIdentifier = ProcessIdentifier::from(args.arg2 as usize);
    let dst_vaddr: PageAligned<VirtualAddress> =
        match PageAligned::from_raw_value(args.arg3 as usize) {
            Ok(vaddr) => vaddr,
            Err(e) => return e.code.into_errno(),
        };

    match do_mcopy(mm, src_pid, src_vaddr, dst_pid, dst_vaddr) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
