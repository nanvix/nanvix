// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::KcallResult,
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

///
/// # Description
///
/// Kernel call handler for controlling process capabilities.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: Encoded capability identifier.
/// - `arg1`: Capability value (nonzero means enabled).
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn capctl(pid: ProcessIdentifier, arg0: u32, arg1: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // Unpack arguments.
    let capability: Capability = match Capability::try_from(arg0) {
        Ok(capability) => capability,
        Err(e) => return KcallResult::Error(e.code.into()),
    };
    let value: bool = arg1 != 0;

    match do_capctl(pm, pid, capability, value) {
        Ok(()) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
