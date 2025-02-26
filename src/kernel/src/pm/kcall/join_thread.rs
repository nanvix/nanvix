// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    self,
    ProcessManager,
};
use ::sys::{
    error::Error,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_join_thread(pid: ProcessIdentifier, tid: ThreadIdentifier) -> Result<usize, Error> {
    ProcessManager::join_thread(pid, tid)
}

pub fn join_thread(arg0: u32, arg1: u32) -> i32 {
    // Unpack kernel call arguments.
    let tid: ThreadIdentifier = ThreadIdentifier::from(arg0 as usize);
    let retval: *mut usize = arg1 as *mut usize;

    let pid: ProcessIdentifier = match ProcessManager::get_pid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    match do_join_thread(pid, tid) {
        Ok(status) => match pm::copy_to_user::<usize>(pid, retval, &status) {
            Ok(_) => 0,
            Err(e) => e.code.into_errno(),
        },
        Err(e) => e.code.into_errno(),
    }
}
