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
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
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
::static_assert::assert_eq_size!(UpdateFileAccessTimeRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl UpdateFileAccessTimeRequest {
    pub const PADDING_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - 2 * mem::size_of::<timespec>();

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, fd: i32, times: &[timespec; 2]) -> Message {
        let message: UpdateFileAccessTimeRequest = UpdateFileAccessTimeRequest {
            fd,
            times: *times,
            _padding: [0; Self::PADDING_SIZE],
        };
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::UpdateFileAccessTimeRequest,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(crate::LINUXD),
            MessageType::Ikc,
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
::static_assert::assert_eq_size!(UpdateFileAccessTimeResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl UpdateFileAccessTimeResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(ret: i32) -> Self {
        Self {
            ret,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, ret: i32) -> Message {
        let message: UpdateFileAccessTimeResponse = UpdateFileAccessTimeResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::UpdateFileAccessTimeResponse,
            message.into_bytes(),
        );
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
