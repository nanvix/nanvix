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

fn do_setegid(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    gid: GroupIdentifier,
) -> Result<(), Error> {
    pm.setegid(pid, gid)
}

pub fn setegid(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    match do_setegid(pm, args.pid, GroupIdentifier::from(args.arg0)) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
