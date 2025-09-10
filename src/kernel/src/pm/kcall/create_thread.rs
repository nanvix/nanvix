// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::{
        KcallArgs,
        KcallResult,
    },
    mm::{
        VirtMemoryManager,
        Vmem,
    },
    pm::{
        self,
        ProcessManager,
    },
};
use ::config::memory_layout::USER_STACK_SIZE;
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
/// Creates a new thread in the calling process.
///
/// # Parameters
///
/// - `pm`: Handler to the process manager.
/// - `mm`: Handler to the virtual memory manager.
/// - `args`: Kernel call arguments containing the thread creation parameters.
///
/// # Returns
///
/// If successful, this function returns the thread identifier of the newly created thread.
/// Otherwise, it returns an error code.
///
pub fn create_thread(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    args: &KcallArgs,
) -> KcallResult {
    // Unpack kernel call arguments.
    let pid: ProcessIdentifier = args.pid;
    let unsafe_thread_create_args: VirtualAddress =
        VirtualAddress::from_raw_value(args.arg0 as usize);

    // Check if thread_create_args does not lie in user space.
    if !Vmem::is_user_region(unsafe_thread_create_args, size_of::<ThreadCreateArgs>()) {
        let reason: &str = "thread_create_args does not lie in user space";
        error!("{reason} (thread_create_args={unsafe_thread_create_args:?})");
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    // Copy thread_create_args from user space to kernel space.
    let mut thread_create_args: ThreadCreateArgs = ThreadCreateArgs::default();
    if let Err(error) = pm::copy_from_user(
        pm,
        args.pid,
        &mut thread_create_args,
        unsafe_thread_create_args.into_raw_value() as *const ThreadCreateArgs,
    ) {
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
    if thread_create_args.user_stack_size < USER_STACK_SIZE {
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
