// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::io::IoPortWidth,
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

fn do_pmio_read(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    port_number: u16,
    port_width: IoPortWidth,
) -> Result<u32, Error> {
    trace!(
        "do_pmio_read(): pid={:?}, port_number={:?}, port_width={:?}",
        pid,
        port_number,
        port_width
    );

    // Check if the process does not have I/O management capabilities.
    if !ProcessManager::has_capability(pid, kcall::Capability::IoManagement)? {
        let reason: &'static str = "process does not have io management capabilities";
        error!("do_pmio_read(): {}", reason);
        return Err(Error::new(ErrorCode::PermissionDenied, &reason));
    }

    pm.read_pmio(pid, port_number, port_width)
}

pub fn pmio_read(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    // Unpack arguments.
    let pid: ProcessIdentifier = args.pid;
    let port_number: u16 = args.arg0 as u16;
    let port_width: IoPortWidth = match IoPortWidth::try_from(args.arg1) {
        Ok(port_width) => port_width,
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };

    // Execute kernel call.
    match do_pmio_read(pm, pid, port_number, port_width) {
        Ok(value) => value as i32,
        Err(e) => e.code.into_errno(),
    }
}
