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
    pm::ProcessManager,
};
use ::sys::{
    error::Error,
    mm::VirtualAddress,
    pm::ThreadIdentifier,
};
use sys::{
    error::ErrorCode,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_create_thread(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    pid: ProcessIdentifier,
    user_wrapper_fn: VirtualAddress,
    user_fn: VirtualAddress,
    user_fn_arg: usize,
) -> Result<ThreadIdentifier, Error> {
    pm.create_thread(mm, pid, user_wrapper_fn, user_fn, user_fn_arg)
}

pub fn create_thread(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    args: &KcallArgs,
) -> KcallResult {
    // Unpack kernel call arguments.
    let user_wrapper_fn: VirtualAddress = VirtualAddress::from_raw_value(args.arg0 as usize);
    let user_fn: VirtualAddress = VirtualAddress::from_raw_value(args.arg1 as usize);
    let user_fn_arg: usize = args.arg2 as usize;

    // Ensure that user function lies within the user address space.
    if !Vmem::is_user_addr(user_fn) {
        let reason: &str = "user function is not within the user address space";
        error!("create_thread(): {} (user_func={:?})", reason, user_fn);
        return KcallResult::Error(ErrorCode::InvalidArgument.into());
    }

    match do_create_thread(pm, mm, args.pid, user_wrapper_fn, user_fn, user_fn_arg) {
        Ok(tid) => KcallResult::Success(Into::<usize>::into(tid).into()),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
