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
        ProcessIdentifier,
        UserIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_setuid(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    uid: UserIdentifier,
) -> Result<(), Error> {
    pm.setuid(pid, uid)
}

pub fn setuid(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    match do_setuid(pm, args.pid, UserIdentifier::from(args.arg0)) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
