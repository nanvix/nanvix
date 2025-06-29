// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::mem;
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::{
    sys_times::tms,
    sys_types::clock_t,
};

//==================================================================================================
// TimesRequest
//==================================================================================================

#[repr(C, packed)]
pub struct TimesRequest {
    pub _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(TimesRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl TimesRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier) -> Result<Message, Error> {
        let message: TimesRequest = TimesRequest::new();
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::TimesRequest, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(crate::LINUXD),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        Ok(message)
    }
}

//==================================================================================================
// TimesResponse
//==================================================================================================

#[repr(C, packed)]
pub struct TimesResponse {
    pub elapsed: clock_t,
    pub buffer: tms,
    pub _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(TimesResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl TimesResponse {
    pub const PADDING_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<clock_t>() - mem::size_of::<tms>();

    fn new(elapsed: clock_t, buffer: tms) -> Self {
        Self {
            elapsed,
            buffer,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, elapsed: clock_t, buffer: tms) -> Message {
        let message: TimesResponse = TimesResponse::new(elapsed, buffer);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::TimesResponse, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(crate::LINUXD),
            MessageReceiver::from(tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );
        message
    }
}
