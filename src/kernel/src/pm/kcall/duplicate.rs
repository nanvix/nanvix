// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
    mm::{
        VirtMemoryManager,
        Vmem,
    },
    pm::{
        self,
        ProcessManager,
    },
};
use ::core::mem::size_of;
use ::sys::{
    error::ErrorCode,
    mm::{
        Address,
        VirtualAddress,
    },
    pm::{
        ProcessIdentifier,
        ThreadCreateArgs,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler for duplicating the calling process. The child process inherits a clone
/// of the caller's address space and starts executing the user-supplied entry function on the
/// user-supplied stack. The operation is refused when the calling process owns one or more
/// special resources (memory-mapped I/O regions, port-mapped I/O ports, event ownerships, or
/// buffered in-flight mailbox messages).
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: Pointer to the [`ThreadCreateArgs`] structure describing the child main thread's
///   entry point and stack, in user space.
///
/// # Returns
///
/// A [`KcallResult`] containing the process identifier of the newly created process on success
/// or the error code on failure.
///
pub fn duplicate(pid: ProcessIdentifier, arg0: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
    // SAFETY: the virtual memory manager is initialized and access is synchronized.
    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };

    // Unpack kernel call arguments.
    let unsafe_args: VirtualAddress = VirtualAddress::from_raw_value(arg0 as usize);

    // Check that the argument lies entirely in user space.
    if !Vmem::is_user_region(unsafe_args, size_of::<ThreadCreateArgs>()) {
        let reason: &str = "duplicate args do not lie in user space";
        error!("{reason} (args={unsafe_args:?})");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Copy the arguments from user space.
    let mut args: ThreadCreateArgs = ThreadCreateArgs::default();
    if let Err(error) = pm::copy_from_user(
        pm,
        pid,
        &mut args,
        unsafe_args.into_raw_value() as *const ThreadCreateArgs,
    ) {
        let reason: &str = "failed to copy duplicate args from user space";
        error!("{reason} (error={:?})", error);
        return KcallResult::Error(error.code.into());
    }

    // Handle the request.
    match pm.duplicate_process(mm, pid, &args) {
        Ok(child_pid) => {
            debug!("process {child_pid:?} duplicated from {pid:?}");
            KcallResult::Success(<i32>::from(child_pid).into())
        },
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
