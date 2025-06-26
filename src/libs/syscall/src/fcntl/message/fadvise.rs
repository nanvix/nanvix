// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::{
    fmt::Debug,
    mem,
};
use ::sys::{
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::sys_types::off_t;

//==================================================================================================
// FileAdvisoryInformationRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileAdvisoryInformationRequest {
    pub fd: i32,
    pub offset: off_t,
    pub len: off_t,
    pub advice: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileAdvisoryInformationRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileAdvisoryInformationRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<off_t>()
        - mem::size_of::<off_t>()
        - mem::size_of::<i32>();

    pub fn new(fd: i32, offset: off_t, len: off_t, advice: i32) -> Self {
        Self {
            fd,
            offset,
            len,
            advice,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        fd: i32,
        offset: off_t,
        len: off_t,
        advice: i32,
    ) -> Message {
        let message: FileAdvisoryInformationRequest =
            FileAdvisoryInformationRequest::new(fd, offset, len, advice);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileAdvisoryInformationRequest,
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
// FileAdvisoryInformationResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileAdvisoryInformationResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileAdvisoryInformationResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileAdvisoryInformationResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    pub fn new(ret: i32) -> Self {
        Self {
            ret,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, ret: i32) -> Message {
        let message: FileAdvisoryInformationResponse = FileAdvisoryInformationResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileAdvisoryInformationResponse,
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
