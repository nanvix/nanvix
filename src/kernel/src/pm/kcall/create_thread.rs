// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallArgs,
    mm::{
        VirtMemoryManager,
        Vmem,
    },
    pm::ProcessManager,
};
use ::core::hint::cold_path;
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

pub fn create_thread(pm: &mut ProcessManager, mm: &mut VirtMemoryManager, args: &KcallArgs) -> i32 {
    // Unpack kernel call arguments.
    let user_wrapper_fn: VirtualAddress = VirtualAddress::from_raw_value(args.arg0 as usize);
    let user_fn: VirtualAddress = VirtualAddress::from_raw_value(args.arg1 as usize);
    let user_fn_arg: usize = args.arg2 as usize;

    // Ensure that user function lies within the user address space.
    if !Vmem::is_user_addr(user_fn) {
        let reason: &str = "user function is not within the user address space";
        error!("create_thread(): {} (user_func={:?})", reason, user_fn);
        return ErrorCode::InvalidArgument.into_errno();
    }

    match do_create_thread(pm, mm, args.pid, user_wrapper_fn, user_fn, user_fn_arg) {
        Ok(tid) => match tid.try_into() {
            Ok(tid) => tid,
            Err(error) => {
                cold_path();
                warn!("do_kcall(): failed to convert tid to i32 (error={:?})", error);
                error.code.into_errno()
            },
        },
        Err(e) => e.code.into_errno(),
    }
}
