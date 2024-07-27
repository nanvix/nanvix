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

fn do_seteuid(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    uid: UserIdentifier,
) -> Result<(), Error> {
    pm.seteuid(pid, uid)
}

pub fn seteuid(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    match do_seteuid(pm, args.pid, UserIdentifier::from(args.arg0)) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
