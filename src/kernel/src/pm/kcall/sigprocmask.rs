// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    mm::Vmem,
    pm::{
        self,
        ProcessManager,
    },
};
use ::core::mem::size_of;
use ::sys::{
    error::ErrorCode,
    mm::VirtualAddress,
    pm::{
        ProcessIdentifier,
        SigSet,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for `sigprocmask()`, which gets and/or modifies the blocked-signal mask of
/// the calling thread.
///
/// # Parameters
///
/// - `caller_pid`: Identifier of the calling process (owner of the `set`/`oldset` buffers).
/// - `caller_tid`: Identifier of the calling thread (owner of the blocked mask).
/// - `arg0`: How to combine `set` with the current mask (`SIG_BLOCK`, `SIG_UNBLOCK`, or
///   `SIG_SETMASK`); consulted only when `set` is non-null.
/// - `arg1`: User-space pointer to the signals to apply, or null to leave the mask unchanged.
/// - `arg2`: User-space pointer that receives the previous mask, or null if not wanted.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn sigprocmask(
    caller_pid: ProcessIdentifier,
    caller_tid: ThreadIdentifier,
    arg0: u32,
    arg1: u32,
    arg2: u32,
) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // The `how` value is a bit-preserving reinterpretation of the raw argument; it is only
    // consulted when `set` is provided.
    let how: i32 = arg0 as i32;
    let set_ptr: usize = arg1 as usize;
    let oldset_ptr: usize = arg2 as usize;

    // Check if the old signal set buffer lies in user space, if provided. Do this before mutating
    // the mask so an invalid output pointer cannot turn the call into a partial success.
    if oldset_ptr != 0 {
        let oldset_addr: VirtualAddress = VirtualAddress::from_raw_value(oldset_ptr);
        if !Vmem::is_user_region(oldset_addr, size_of::<SigSet>()) {
            let reason: &str = "old signal set buffer does not lie in user space";
            error!("{reason} (oldset={oldset_addr:?})");
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        }
    }

    // Read the signals to apply from user space, if provided.
    let set: Option<SigSet> = if set_ptr != 0 {
        let set_addr: VirtualAddress = VirtualAddress::from_raw_value(set_ptr);
        if !Vmem::is_user_region(set_addr, size_of::<SigSet>()) {
            let reason: &str = "signal set does not lie in user space";
            error!("{reason} (set={set_addr:?})");
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        }
        let mut value: SigSet = 0;
        if let Err(error) = pm::copy_from_user_addr(pm, caller_pid, &mut value, set_addr) {
            error!("failed to copy signal set from user space (error={error:?})");
            return KcallResult::Error(error.code.into());
        }
        Some(value)
    } else {
        None
    };

    // Apply the change to the calling thread's mask, capturing the previous mask.
    let old: SigSet = match pm.sigprocmask(caller_tid, how, set) {
        Ok(old) => old,
        Err(error) => {
            error!("{error:?}");
            return KcallResult::Error(error.code.into());
        },
    };

    // Report the previous mask through `oldset`, if requested.
    if oldset_ptr != 0 {
        let oldset_addr: VirtualAddress = VirtualAddress::from_raw_value(oldset_ptr);
        if let Err(error) = pm::copy_to_user_addr(pm, caller_pid, oldset_addr, &old) {
            error!("failed to copy old signal set to user space (error={error:?})");
            return KcallResult::Error(error.code.into());
        }
    }

    KcallResult::ok()
}
