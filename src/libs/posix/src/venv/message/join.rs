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
// JoinEnvRequest
//==================================================================================================

#[repr(C, packed)]
pub struct JoinEnvRequest {
    pub env: VirtualEnvironmentIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(JoinEnvRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl JoinEnvRequest {
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
        let message: JoinEnvRequest = JoinEnvRequest::new(env);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::JoinEnvRequest, message.into_bytes());
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

impl Debug for JoinEnvRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let env: VirtualEnvironmentIdentifier = self.env;
        write!(f, "{{ env: {:?} }}", env)
    }
}

//==================================================================================================
// JoinEnvResponse
//==================================================================================================

#[repr(C, packed)]
pub struct JoinEnvResponse {
    pub env: VirtualEnvironmentIdentifier,
    pub _padding: [u8; Self::PADDING_SIZE],
}

impl JoinEnvResponse {
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
        let message: JoinEnvResponse = JoinEnvResponse::new(env);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::JoinEnvResponse,
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
