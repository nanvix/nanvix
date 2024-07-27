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
    GroupIdentifier,
    ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_getegid(pm: &ProcessManager, pid: ProcessIdentifier) -> Result<GroupIdentifier, Error> {
    pm.getegid(pid)
}

pub fn getegid(pm: &ProcessManager, args: &KcallArgs) -> i32 {
    match do_getegid(pm, args.pid) {
        Ok(egid) => egid.into(),
        Err(e) => e.code.into_errno(),
    }
}
