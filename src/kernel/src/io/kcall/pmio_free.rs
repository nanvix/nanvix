// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::io::AnyIoPort,
    kcall::KcallResult,
    pm::ProcessManager,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        Capability,
        ProcessIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_pmio_free(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    port_number: u16,
) -> Result<(), Error> {
    trace!("pid={:?}, portnum={:?}", pid, port_number);

    // Check if the process does not have I/O management capabilities.
    if !pm.has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have io management capabilities";
        error!("{reason}");
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    let _port: AnyIoPort = pm.detach_pmio(pid, port_number)?;

    Ok(())
}

///
/// # Description
///
/// Kernel call handler for releasing a port-mapped I/O port.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: Port number to release (lower 16 bits used).
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn pmio_free(pid: ProcessIdentifier, arg0: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // Unpack arguments.
    let port_number: u16 = arg0 as u16;

    // Execute kernel call.
    match do_pmio_free(pm, pid, port_number) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
