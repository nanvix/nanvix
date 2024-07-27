// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallArgs,
    pm::process::ProcessManager,
};
use ::kcall::{
    Error,
    ProcessIdentifier,
    UserIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_geteuid(pm: &ProcessManager, pid: ProcessIdentifier) -> Result<UserIdentifier, Error> {
    pm.geteuid(pid)
}

pub fn geteuid(pm: &ProcessManager, args: &KcallArgs) -> i32 {
    match do_geteuid(pm, args.pid) {
        Ok(euid) => euid.into(),
        Err(e) => e.code.into_errno(),
    }
}
