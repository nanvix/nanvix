// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallArgs,
    pm::ProcessManager,
};
use ::kcall::{
    Capability,
    Error,
    ErrorCode,
    ProcessIdentifier,
    UserIdentifier,
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
    // Checks if the process has enough privileges.
    if pm.geteuid(pid)? != UserIdentifier::ROOT {
        let reason: &str = "permission denied";
        error!("do_capctl: {}", reason);
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    pm.capctl(pid, capability, value)
}

pub fn capctl(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    // Unpack arguments.
    let capability: Capability = match Capability::try_from(args.arg0) {
        Ok(capability) => capability,
        Err(e) => return e.code.into_errno(),
    };
    let value: bool = args.arg1 != 0;

    match do_capctl(pm, args.pid, capability, value) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
