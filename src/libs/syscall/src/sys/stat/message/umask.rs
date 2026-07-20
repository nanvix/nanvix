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
use ::sysapi::sys_types::mode_t;

//==================================================================================================
// FileCreationMaskRequest
//==================================================================================================

/// Request to set the calling process's file mode creation mask.
#[derive(Debug)]
#[repr(C, packed)]
pub struct FileCreationMaskRequest {
    /// New file mode creation mask.
    pub mask: mode_t,
    /// Padding.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileCreationMaskRequest, SystemCallMessage::PAYLOAD_SIZE);

impl FileCreationMaskRequest {
    /// Size of padding.
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<mode_t>();

    fn new(mask: mode_t) -> Self {
        Self {
            mask,
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
        mask: mode_t,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let request: Self = Self::new(mask);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::FileCreationMaskRequest,
            request.into_bytes(),
        );
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}

//==================================================================================================
// FileCreationMaskResponse
//==================================================================================================

/// Response carrying the calling process's previous file mode creation mask.
#[derive(Debug)]
#[repr(C, packed)]
pub struct FileCreationMaskResponse {
    /// Previous file mode creation mask.
    pub mask: mode_t,
    /// Padding.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileCreationMaskResponse, SystemCallMessage::PAYLOAD_SIZE);

impl FileCreationMaskResponse {
    /// Size of padding.
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<mode_t>();

    fn new(mask: mode_t) -> Self {
        Self {
            mask,
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
        mask: mode_t,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let response: Self = Self::new(mask);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::FileCreationMaskResponse,
            response.into_bytes(),
        );
        Message::new(
            MessageSender::new(source, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}
