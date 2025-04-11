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

fn do_seteuid(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    uid: UserIdentifier,
) -> Result<(), Error> {
    pm.seteuid(pid, uid)
}

pub fn seteuid(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    match do_seteuid(pm, args.pid, UserIdentifier::from(args.arg0)) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
