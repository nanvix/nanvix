// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    venv::VirtualEnvironmentIdentifier,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::{
    fmt::Debug,
    mem,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
};

//==================================================================================================
// LeaveEnvRequest
//==================================================================================================

#[repr(C, packed)]
pub struct LeaveEnvRequest {
    pub env: VirtualEnvironmentIdentifier,
    pub _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(LeaveEnvRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl LeaveEnvRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<u32>();

    pub fn new(env: VirtualEnvironmentIdentifier) -> Self {
        Self {
            env,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, env: VirtualEnvironmentIdentifier) -> Message {
        let message: LeaveEnvRequest = LeaveEnvRequest::new(env);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::LeaveEnvRequest,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            pid,
            crate::LINUXD,
            nvx::ipc::MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        message
    }
}

impl Debug for LeaveEnvRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let env: VirtualEnvironmentIdentifier = self.env;
        write!(f, "{{ env: {:?} }}", env)
    }
}

//==================================================================================================
// LeaveEnvResponse
//==================================================================================================

#[repr(C, packed)]
pub struct LeaveEnvResponse {
    pub env: VirtualEnvironmentIdentifier,
    pub _padding: [u8; Self::PADDING_SIZE],
}

impl LeaveEnvResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<u32>();

    pub fn new(env: VirtualEnvironmentIdentifier) -> Self {
        Self {
            env,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, env: VirtualEnvironmentIdentifier) -> Message {
        let message: LeaveEnvResponse = LeaveEnvResponse::new(env);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::LeaveEnvResponse,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            crate::LINUXD,
            pid,
            nvx::ipc::MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        message
    }
}
