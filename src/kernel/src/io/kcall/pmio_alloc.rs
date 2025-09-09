// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        io::{
            AnyIoPort,
            IoPortType,
        },
        Hal,
    },
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

fn do_pmio_alloc(
    hal: &mut Hal,
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    port_type: IoPortType,
    port_number: u16,
) -> Result<(), Error> {
    trace!("pid={:?}, port_number={:?}, port_type={:?}", pid, port_number, port_type);

    // Check if the process does not have I/O management capabilities.
    if !pm.has_capability(pid, Capability::IoManagement)? {
        let reason: &'static str = "process does not have io management capabilities";
        error!("{reason}");
        return Err(Error::new(ErrorCode::PermissionDenied, reason));
    }

    let port: AnyIoPort = match port_type {
        IoPortType::ReadOnly => AnyIoPort::ReadOnly(hal.ioports.allocate_read_only(port_number)?),
        IoPortType::WriteOnly => {
            AnyIoPort::WriteOnly(hal.ioports.allocate_write_only(port_number)?)
        },
        IoPortType::ReadWrite => {
            AnyIoPort::ReadWrite(hal.ioports.allocate_read_write(port_number)?)
        },
    };

    pm.attach_pmio(pid, port)
}

pub fn pmio_alloc(hal: &mut Hal, pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    // Unpack arguments.
    let pid: ProcessIdentifier = args.pid;
    let port_number: u16 = args.arg0 as u16;
    let port_type: IoPortType = match IoPortType::try_from(args.arg1) {
        Ok(port_type) => port_type,
        Err(e) => return KcallResult::Error(e.code.into()),
    };

    // Execute kernel call.
    match do_pmio_alloc(hal, pm, pid, port_type, port_number) {
        Ok(_) => KcallResult::ok(),
        Err(e) => KcallResult::Error(e.code.into()),
    }
}
