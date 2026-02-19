// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::io::IoPortWidth,
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

fn do_pmio_read(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    port_number: u16,
    port_width: IoPortWidth,
) -> Result<u32, Error> {
    trace!("pid={:?}, port_number={:?}, port_width={:?}", pid, port_number, port_width);

    // Check if the process does not have I/O management capabilities.
    if !pm.has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have io management capabilities";
        error!("{reason}");
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    pm.read_pmio(pid, port_number, port_width)
}

///
/// # Description
///
/// Kernel call handler for reading from a port-mapped I/O port.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `arg0`: Port number to read from (lower 16 bits used).
/// - `arg1`: Encoded port width.
///
/// # Returns
///
/// A [`KcallResult`] containing the read value on success or the error code.
///
pub fn pmio_read(pid: ProcessIdentifier, arg0: u32, arg1: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    // Unpack arguments.
    let port_number: u16 = arg0 as u16;
    let port_width: IoPortWidth = match IoPortWidth::try_from(arg1) {
        Ok(port_width) => port_width,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    // Execute kernel call.
    match do_pmio_read(pm, pid, port_number, port_width) {
        Ok(value) => KcallResult::Success(value.into()),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
