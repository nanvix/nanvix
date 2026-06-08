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
use sysapi::time::timespec;

//==================================================================================================
// UpdateFileAccessTimeRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct UpdateFileAccessTimeRequest {
    pub fd: i32,
    pub times: [timespec; 2],
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(UpdateFileAccessTimeRequest, SystemCallMessage::PAYLOAD_SIZE);

impl UpdateFileAccessTimeRequest {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - 2 * mem::size_of::<timespec>();

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        fd: i32,
        times: &[timespec; 2],
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: UpdateFileAccessTimeRequest = UpdateFileAccessTimeRequest {
            fd,
            times: *times,
            _padding: [0; Self::PADDING_SIZE],
        };
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::UpdateFileAccessTimeRequest,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(destination),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}

//==================================================================================================
// UpdateFileAccessTimeResponse
//==================================================================================================

#[repr(C, packed)]
pub struct UpdateFileAccessTimeResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(UpdateFileAccessTimeResponse, SystemCallMessage::PAYLOAD_SIZE);

impl UpdateFileAccessTimeResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(ret: i32) -> Self {
        Self {
            ret,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        ret: i32,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: UpdateFileAccessTimeResponse = UpdateFileAccessTimeResponse::new(ret);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::UpdateFileAccessTimeResponse,
            message.into_bytes(),
        );
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
