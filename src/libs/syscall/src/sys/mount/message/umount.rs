// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::{
        MessageDeserializer,
        MessagePartitioner,
        MessageSerializer,
        SystemCallMessagePart,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::core::{
    convert::TryInto,
    mem,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
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
use sysapi::limits::PATH_MAX;

//==================================================================================================
// UmountRequest
//==================================================================================================

/// Request message for the `umount()` system call.
///
/// Layout: `target_len (u32) | target`
#[derive(Debug)]
pub struct UmountRequest {
    /// Target mount point to unmount.
    pub target: String,
}

impl UmountRequest {
    const SIZE_OF_TARGET_LEN: usize = mem::size_of::<u32>();
    const OFFSET_OF_TARGET_LEN: usize = 0;
    const OFFSET_OF_TARGET: usize = Self::SIZE_OF_TARGET_LEN;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::SIZE_OF_TARGET_LEN + PATH_MAX;

    pub fn new(target: String) -> Result<Self, Error> {
        if target.len() > PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "target path too long"));
        }
        Ok(Self { target })
    }
}

impl MessageSerializer for UmountRequest {
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        let target_bytes: &[u8] = self.target.as_bytes();

        buffer.extend_from_slice(&(target_bytes.len() as u32).to_ne_bytes());
        buffer.extend_from_slice(target_bytes);

        buffer
    }
}

impl MessageDeserializer for UmountRequest {
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < Self::OFFSET_OF_TARGET {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }
        if bytes.len() > Self::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too long"));
        }

        let target_len: usize = u32::from_ne_bytes(
            bytes[Self::OFFSET_OF_TARGET_LEN..Self::OFFSET_OF_TARGET]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid target_len"))?,
        ) as usize;

        if bytes.len() < Self::OFFSET_OF_TARGET + target_len {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short for target"));
        }
        if target_len > PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "target path too long"));
        }

        let target: String = String::from_utf8(
            bytes[Self::OFFSET_OF_TARGET..Self::OFFSET_OF_TARGET + target_len].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid target path"))?;

        Ok(Self { target })
    }
}

impl MessagePartitioner for UmountRequest {
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; SystemCallMessagePart::PAYLOAD_SIZE],
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Message, Error> {
        SystemCallMessagePart::build_request(
            tid,
            SystemCallMessageHeader::HostUmountRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
            destination,
            message_type,
        )
    }
}

//==================================================================================================
// UmountResponse
//==================================================================================================

#[repr(C, packed)]
pub struct UmountResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(UmountResponse, SystemCallMessage::PAYLOAD_SIZE);

impl UmountResponse {
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
        let message: UmountResponse = UmountResponse::new(ret);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::HostUmountResponse,
            message.into_bytes(),
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
