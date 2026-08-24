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
use ::config::memory_layout::USER_STACK_MIN_SIZE;
use ::core::mem::size_of;
use ::sys::{
    error::ErrorCode,
    mm::VirtualAddress,
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
/// Kernel call handler for creating a new thread in the calling process.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: Pointer to the thread creation arguments in user space.
///
/// # Returns
///
/// A [`KcallResult`] containing the thread identifier of the newly created thread on success or
/// the error code.
///
pub fn create_thread(pid: ProcessIdentifier, arg0: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
    // SAFETY: the virtual memory manager is initialized and access is synchronized.
    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };

    // Unpack kernel call arguments.
    let unsafe_thread_create_args: VirtualAddress = VirtualAddress::from_raw_value(arg0 as usize);

    // Check if thread_create_args does not lie in user space.
    if !Vmem::is_user_region(unsafe_thread_create_args, size_of::<ThreadCreateArgs>()) {
        let reason: &str = "thread_create_args does not lie in user space";
        error!("{reason} (thread_create_args={unsafe_thread_create_args:?})");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Copy thread_create_args from user space to kernel space.
    let mut thread_create_args: ThreadCreateArgs = ThreadCreateArgs::default();
    if let Err(error) =
        pm::copy_from_user_addr(pm, pid, &mut thread_create_args, unsafe_thread_create_args)
    {
        let reason: &str = "failed to copy thread_create_args from user space";
        error!("{reason:?} (error={:?})", error);
        return KcallResult::Error(error.code.into());
    }

    // Check if the user wrapper function does not lie within the user address space.
    if !Vmem::is_user_addr(thread_create_args.user_fn) {
        let reason: &str = "user function does not lie within the user address space";
        error!("{reason} (user_fn={:?})", thread_create_args.user_fn);
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Check if the user stack does not lie within the user address space.
    if !Vmem::is_user_region(thread_create_args.user_stack_base, thread_create_args.user_stack_size)
    {
        let reason: &str = "user stack does not lie within the user address space";
        error!(
            "create_thread(): {reason} (user_stack_base={:?}, user_stack_size={})",
            thread_create_args.user_stack_base, thread_create_args.user_stack_size
        );
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Check if user stack size is too small.
    if thread_create_args.user_stack_size < USER_STACK_MIN_SIZE {
        let reason: &str = "user stack size is too small";
        error!(
            "create_thread(): {reason} (user_stack_size={})",
            thread_create_args.user_stack_size
        );
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Check if the base address of user-space thread data area does not lie in the user space.
    if let Some(user_tda) = thread_create_args.user_tda {
        if !Vmem::is_user_addr(user_tda) {
            let reason: &str =
                "user-space thread data area does not lie within the user address space";
            error!("{reason} (user_tcb={:?})", thread_create_args.user_tda);
            return KcallResult::Error(ErrorCode::InvalidArgument.into());
        }
    }

    // Handle thread creation.
    match pm.create_thread(mm, pid, &thread_create_args) {
        Ok(tid) => {
            debug!("thread {tid:?} created");
            KcallResult::Success(<i32>::from(tid).into())
        },
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
