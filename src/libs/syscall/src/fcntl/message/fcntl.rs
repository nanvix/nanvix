// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageKind,
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
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// FileControlRequest
//==================================================================================================A

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileControlRequest {
    pub fd: i32,
    pub cmd: i32,
    pub arg: c_int,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileControlRequest, SystemCallMessage::PAYLOAD_SIZE);

impl FileControlRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<i32>()
        - mem::size_of::<c_int>();

    pub fn new(fd: i32, cmd: i32, arg: c_int) -> Self {
        Self {
            fd,
            cmd,
            arg,
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
        fd: i32,
        cmd: i32,
        arg: c_int,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: FileControlRequest = FileControlRequest::new(fd, cmd, arg);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::FileControlRequest, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}

//==================================================================================================
// FileControlResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileControlResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileControlResponse, SystemCallMessage::PAYLOAD_SIZE);

impl FileControlResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    pub fn new(ret: i32) -> Self {
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
        let message: FileControlResponse = FileControlResponse::new(ret);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::FileControlResponse,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::new(source, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}
