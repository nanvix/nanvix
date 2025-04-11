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

fn do_setgid(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    gid: GroupIdentifier,
) -> Result<(), Error> {
    pm.setgid(pid, gid)
}

pub fn setgid(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    match do_setgid(pm, args.pid, GroupIdentifier::from(args.arg0)) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
