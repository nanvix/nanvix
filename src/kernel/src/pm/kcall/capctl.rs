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
    pm::ProcessManager,
};
use ::sys::{
    error::Error,
    pm::{
        Capability,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_capctl(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    capability: Capability,
    value: bool,
) -> Result<(), Error> {
    trace!("pid={:?}, capability={:?}, value={:?}", pid, capability, value);

    //FIXME: check if process has enough privileges to change capabilities.

    pm.capctl(pid, capability, value)
}

pub fn capctl(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Unpack arguments.
    let capability: Capability = match Capability::try_from(args.arg0) {
        Ok(capability) => capability,
        Err(e) => return KcallResult::Error(e.code.into()),
    };
    let value: bool = args.arg1 != 0;

    match do_capctl(pm, args.pid, capability, value) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
