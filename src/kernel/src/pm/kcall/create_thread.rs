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
    user_func: VirtualAddress,
) -> Result<ThreadIdentifier, Error> {
    pm.create_thread(mm, pid, user_func)
}

pub fn create_thread(pm: &mut ProcessManager, mm: &mut VirtMemoryManager, args: &KcallArgs) -> i32 {
    // Unpack kernel call arguments.
    let user_func: VirtualAddress = VirtualAddress::from_raw_value(args.arg0 as usize);

    // Ensure that user function lies within the user address space.
    if !Vmem::is_user_addr(user_func) {
        let reason: &str = "user function is not within the user address space";
        error!("create_thread(): {} (user_func={:?})", reason, user_func);
        return ErrorCode::InvalidArgument.into_errno();
    }

    match do_create_thread(pm, mm, args.pid, user_func) {
        Ok(tid) => tid.into(),
        Err(e) => e.code.into_errno(),
    }
}
