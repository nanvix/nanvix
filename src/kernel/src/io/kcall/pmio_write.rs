// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::io::IoPortWidth,
    kcall::{
        KcallArgs,
        KcallResult,
    },
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
    trace!("pid={pid:?}, port_number={port_number:?}, port_width={port_width:?}, value={value:?}");

    // Check if the process does not have I/O management capabilities.
    if !pm.has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have io management capabilities";
        error!("{reason}");
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    pm.write_pmio(pid, port_number, port_width, value)
}

pub fn pmio_write(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Unpack arguments.
    let pid: ProcessIdentifier = args.pid;
    let port_number: u16 = args.arg0 as u16;
    let port_width: IoPortWidth = match IoPortWidth::try_from(args.arg1) {
        Ok(port_width) => port_width,
        Err(_) => return KcallResult::Error(ErrorCode::InvalidArgument.into()),
    };
    let value: u32 = args.arg2;

    // Execute kernel call.
    match do_pmio_write(pm, pid, port_number, port_width, value) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
