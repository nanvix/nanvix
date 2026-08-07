// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        Address,
        VirtualAddress,
    },
    kcall::KcallResult,
    mm::Vmem,
    pm::ProcessManager,
};
use ::arch::mem::PAGE_SIZE;
use ::sys::{
    error::ErrorCode,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel-call handler for making modified user memory visible to instruction fetches.
///
/// The kernel translates each user page and performs cache maintenance through its identity-mapped
/// physical alias. This keeps privileged cache instructions out of EL0, where some hypervisors do
/// not implement them even when `SCTLR_EL1.UCI` permits their use.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process.
/// - `arg0`: Start of the modified user-memory range.
/// - `arg1`: Length of the modified range in bytes.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn sync_instruction_cache(caller_pid: ProcessIdentifier, arg0: u32, arg1: u32) -> KcallResult {
    let start: VirtualAddress = VirtualAddress::new(arg0 as usize);
    let len: usize = arg1 as usize;

    if len == 0 {
        return KcallResult::ok();
    }
    if !Vmem::is_user_region(start, len) {
        error!("instruction-cache range does not lie entirely in user space");
        return KcallResult::Error(ErrorCode::BadAddress.into());
    }

    let end: usize = match start.into_raw_value().checked_add(len) {
        Some(end) => end,
        None => return KcallResult::Error(ErrorCode::BadAddress.into()),
    };
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };
    let mut current: usize = start.into_raw_value();

    while current < end {
        let vaddr: VirtualAddress = VirtualAddress::new(current);
        match pm.user_vaddr_to_paddr(caller_pid, vaddr) {
            Ok(_) => {},
            Err(error) => {
                error!(
                    "failed to translate instruction-cache range (vaddr={current:#x}, \
                     error={error:?})"
                );
                return KcallResult::Error(error.code.into());
            },
        };
        let page_remaining: usize = PAGE_SIZE - (current % PAGE_SIZE);
        let chunk_len: usize = (end - current).min(page_remaining);
        current += chunk_len;
    }

    // SAFETY: every page in the range was validated above. The kernel call runs in the calling
    // process's address space, so cache maintenance uses the same virtual aliases as execution.
    unsafe {
        ::arch::cpu::synchronize_instruction_cache(start.into_raw_value() as *const u8, len);
    }

    KcallResult::ok()
}
