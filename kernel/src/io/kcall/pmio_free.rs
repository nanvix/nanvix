// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::io::AnyIoPort,
    kcall::KcallArgs,
    pm::ProcessManager,
};
use ::kcall::{
    Error,
    ErrorCode,
    ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_pmio_free(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    port_number: u16,
) -> Result<(), Error> {
    trace!("do_pmio_free(): pid={:?}, portnum={:?}", pid, port_number);

    // Check if the process does not have I/O management capabilities.
    if !ProcessManager::has_capability(pid, kcall::Capability::IoManagement)? {
        let reason: &'static str = "process does not have io management capabilities";
        error!("do_pmio_free(): {}", reason);
        return Err(Error::new(ErrorCode::PermissionDenied, &reason));
    }

    let _port: AnyIoPort = pm.detach_pmio(pid, port_number)?;

    Ok(())
}

pub fn pmio_free(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    // Unpack arguments.
    let pid: ProcessIdentifier = args.pid;
    let port_number: u16 = args.arg0 as u16;

    // Execute kernel call.
    match do_pmio_free(pm, pid, port_number) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
