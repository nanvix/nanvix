// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallArgs,
    mm::VirtMemoryManager,
    pm::{
        self,
        ProcessManager,
    },
};
use ::sys::{
    error::Error,
    pm::ThreadIdentifier,
};
use sys::pm::ProcessIdentifier;

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_join_thread(
    pm: &mut ProcessManager,
    mm: &mut VirtMemoryManager,
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
) -> Result<usize, Error> {
    pm.join_thread(mm, pid, tid)
}

pub fn join_thread(pm: &mut ProcessManager, mm: &mut VirtMemoryManager, args: &KcallArgs) -> i32 {
    // Unpack kernel call arguments.
    let tid: ThreadIdentifier = ThreadIdentifier::from(args.arg0 as usize);
    let retval: *mut usize = args.arg1 as *mut usize;

    match do_join_thread(pm, mm, args.pid, tid) {
        Ok(status) => match pm::copy_to_user::<usize>(args.pid, retval, &status) {
            Ok(_) => 0,
            Err(e) => e.code.into_errno(),
        },
        Err(e) => e.code.into_errno(),
    }
}
