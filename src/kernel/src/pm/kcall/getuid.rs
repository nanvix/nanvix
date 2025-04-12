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

fn do_getuid(pm: &ProcessManager, pid: ProcessIdentifier) -> Result<UserIdentifier, Error> {
    pm.getuid(pid)
}

pub fn getuid(pm: &ProcessManager, args: &KcallArgs) -> KcallResult {
    match do_getuid(pm, args.pid) {
        Ok(uid) => KcallResult::Success(Into::<usize>::into(uid).into()),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
