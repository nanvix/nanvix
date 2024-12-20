// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    venv::{
        message::LeaveEnvRequest,
        VirtualEnvironmentIdentifier,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::Error,
};
use nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn leave(env: VirtualEnvironmentIdentifier) -> Result<(), Error> {
    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;
    let request: Message = LeaveEnvRequest::build(pid, env);

    // Send request.
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Parse response.
    if response.status != 0 {
        return Err(Error::new(
            ::nvx::sys::error::ErrorCode::try_from(response.status)?,
            "failed to leave()",
        ));
    } else {
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            Ok(message) => match message.header {
                LinuxDaemonMessageHeader::LeaveEnvResponse => Ok(()),
                _ => Err(Error::new(ErrorCode::InvalidMessage, "failed to parse leave() response")),
            },
            Err(e) => Err(e),
        }
    }
}
