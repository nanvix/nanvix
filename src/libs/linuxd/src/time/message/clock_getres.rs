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
// ClockResolutionRequest
//==================================================================================================

#[repr(C, packed)]
pub struct ClockResolutionRequest {
    pub clock_id: clockid_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(ClockResolutionRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ClockResolutionRequest {
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
        let message: ClockResolutionRequest = ClockResolutionRequest::new(clock_id);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::GetClockResolutionRequest,
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

impl Debug for ClockResolutionRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let clock_id: clockid_t = self.clock_id;
        write!(f, "{{ clock_id: {:?} }}", clock_id)
    }
}

//==================================================================================================
// ClockGetResolutionResponse
//==================================================================================================

#[repr(C, packed)]
pub struct ClockGetResolutionResponse {
    pub res: timespec,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(ClockGetResolutionResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ClockGetResolutionResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<timespec>();

    pub fn new(res: timespec) -> Self {
        Self {
            res,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, res: timespec) -> Message {
        let message: ClockGetResolutionResponse = ClockGetResolutionResponse::new(res);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::GetClockResolutionResponse,
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
