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

fn do_pmio_write(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    port_number: u16,
    port_width: IoPortWidth,
    value: u32,
) -> Result<(), Error> {
    trace!(
        "do_pmio_write(): pid={:?}, port_number={:?}, port_width={:?}, value={:?}",
        pid,
        port_number,
        port_width,
        value
    );

    // Check if the process does not have I/O management capabilities.
    if !ProcessManager::has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have io management capabilities";
        error!("do_pmio_write(): {}", reason);
        return Err(Error::new(ErrorCode::PermissionDenied, &reason));
    }

    pm.write_pmio(pid, port_number, port_width, value)
}

pub fn pmio_write(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    // Unpack arguments.
    let pid: ProcessIdentifier = args.pid;
    let port_number: u16 = args.arg0 as u16;
    let port_width: IoPortWidth = match IoPortWidth::try_from(args.arg1) {
        Ok(port_width) => port_width,
        Err(_) => return ErrorCode::InvalidArgument.into_errno(),
    };
    let value: u32 = args.arg2 as u32;

    // Execute kernel call.
    match do_pmio_write(pm, pid, port_number, port_width, value) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}
