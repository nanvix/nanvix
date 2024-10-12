// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    venv::{
        message::{
            JoinEnvRequest,
            JoinEnvResponse,
        },
        VirtualEnvironmentIdentifier,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn join(env: VirtualEnvironmentIdentifier) -> Result<VirtualEnvironmentIdentifier, Error> {
    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;
    let request: Message = JoinEnvRequest::build(pid, env);

    // Send request.
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Parse response.
    if response.status != 0 {
        Err(Error::new(
            ::nvx::sys::error::ErrorCode::try_from(response.status)?,
            "failed to join()",
        ))
    } else {
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            Ok(message) => match message.header {
                LinuxDaemonMessageHeader::JoinEnvResponse => {
                    let response: JoinEnvResponse = JoinEnvResponse::from_bytes(message.payload);
                    Ok(response.env)
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "failed to parse join() response")),
            },
            Err(e) => Err(e),
        }
    }
}
