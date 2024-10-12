// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    time::{
        clockid_t,
        timespec,
    },
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
// GetClockTimeRequest
//==================================================================================================

#[repr(C, packed)]
pub struct GetClockTimeRequest {
    pub clock_id: clockid_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(GetClockTimeRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl GetClockTimeRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<clockid_t>();

    pub fn new(clock_id: clockid_t) -> Self {
        Self {
            clock_id,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, clock_id: clockid_t) -> Message {
        let message: GetClockTimeRequest = GetClockTimeRequest::new(clock_id);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::GetClockTimeRequest,
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

impl Debug for GetClockTimeRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let clock_id: clockid_t = self.clock_id;
        write!(f, "{{ clock_id: {} }}", clock_id)
    }
}

//==================================================================================================
// GetClockTimeResponse
//==================================================================================================

#[repr(C, packed)]
pub struct GetClockTimeResponse {
    pub tp: timespec,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(GetClockTimeResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl GetClockTimeResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<timespec>();

    pub fn new(tp: timespec) -> Self {
        Self {
            tp,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, tp: timespec) -> Message {
        let message: GetClockTimeResponse = GetClockTimeResponse::new(tp);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::GetClockTimeResponse,
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
