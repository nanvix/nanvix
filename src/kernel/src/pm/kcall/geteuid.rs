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

fn do_geteuid(pm: &ProcessManager, pid: ProcessIdentifier) -> Result<UserIdentifier, Error> {
    pm.geteuid(pid)
}

pub fn geteuid(pm: &ProcessManager, args: &KcallArgs) -> KcallResult {
    match do_geteuid(pm, args.pid) {
        Ok(euid) => KcallResult::Success(Into::<usize>::into(euid).into()),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
