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

fn do_getgid(pm: &ProcessManager, pid: ProcessIdentifier) -> Result<GroupIdentifier, Error> {
    pm.getgid(pid)
}

pub fn getgid(pm: &ProcessManager, args: &KcallArgs) -> i32 {
    match do_getgid(pm, args.pid) {
        Ok(gid) => gid.into(),
        Err(e) => e.code.into_errno(),
    }
}
