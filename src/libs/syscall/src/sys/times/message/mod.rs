// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageHeader,
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
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use sysapi::{
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
::static_assert::assert_eq_size!(TimesRequest, SystemCallMessage::PAYLOAD_SIZE);

impl TimesRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Message, Error> {
        let message: TimesRequest = TimesRequest::new();
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::TimesRequest, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(destination),
            message_type,
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
::static_assert::assert_eq_size!(TimesResponse, SystemCallMessage::PAYLOAD_SIZE);

impl TimesResponse {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<clock_t>() - mem::size_of::<tms>();

    fn new(elapsed: clock_t, buffer: tms) -> Self {
        Self {
            elapsed,
            buffer,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        elapsed: clock_t,
        buffer: tms,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: TimesResponse = TimesResponse::new(elapsed, buffer);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::TimesResponse, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(source),
            MessageReceiver::from(tid),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}
