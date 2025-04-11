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
    pm::process::ProcessManager,
};
use ::sys::{
    error::Error,
    pm::{
        GroupIdentifier,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_getgid(pm: &ProcessManager, pid: ProcessIdentifier) -> Result<GroupIdentifier, Error> {
    pm.getgid(pid)
}

pub fn getgid(pm: &ProcessManager, args: &KcallArgs) -> KcallResult {
    match do_getgid(pm, args.pid) {
        Ok(gid) => KcallResult::Success(Into::<usize>::into(gid).into()),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
