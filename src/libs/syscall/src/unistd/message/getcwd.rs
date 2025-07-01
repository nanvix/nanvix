// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::{
        LinuxDaemonMessagePart,
        MessageDeserializer,
        MessagePartitioner,
        MessageSerializer,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::{
    string::{
        String,
        ToString,
    },
    vec::Vec,
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
    pm::ThreadIdentifier,
};
use sysapi::limits::PATH_MAX;

//==================================================================================================
// GetCurrentWorkingDirectoryRequest
//==================================================================================================

#[repr(C, packed)]
pub struct GetCurrentWorkingDirectoryRequest {
    _padding: [u8; Self::PADDING_SIZE],
}

impl GetCurrentWorkingDirectoryRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { core::mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { core::mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier) -> Message {
        let message: GetCurrentWorkingDirectoryRequest = GetCurrentWorkingDirectoryRequest::new();
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            crate::LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryRequest,
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
// GetCurrentWorkingDirectoryResponse
//==================================================================================================

#[derive(Debug)]
pub struct GetCurrentWorkingDirectoryResponse {
    pub cwd: String,
}

impl GetCurrentWorkingDirectoryResponse {
    /// Size of `cwd.len()` field.
    const SIZE_OF_CWD_LEN: usize = core::mem::size_of::<u32>();
    /// Offset of `cwd.len()` field.
    const OFFSET_OF_CWD_LEN: usize = 0;
    /// Offset of `cwd` field.
    const OFFSET_OF_CWD: usize = Self::OFFSET_OF_CWD_LEN + Self::SIZE_OF_CWD_LEN;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::SIZE_OF_CWD_LEN + PATH_MAX;

    /// Creates a new response for the `getcwd` system call.
    pub fn new(cwd: &str) -> Result<Self, Error> {
        // Check if `cwd` is too long.
        if cwd.len() > PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidArgument, "cwd is too long"));
        }

        Ok(Self {
            cwd: cwd.to_string(),
        })
    }
}

impl MessageSerializer for GetCurrentWorkingDirectoryResponse {
    /// Serializes a response message for the `getcwd()` system call.
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        let cwd_bytes: &[u8] = self.cwd.as_bytes();

        // Serialize `cwd.len` field.
        bytes.extend_from_slice(&(cwd_bytes.len() as u32).to_le_bytes());
        // Serialize `cwd` field.
        bytes.extend_from_slice(cwd_bytes);

        bytes
    }
}

impl MessageDeserializer for GetCurrentWorkingDirectoryResponse {
    /// Deserializes a response message for the `getcwd()` system call.
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if message is too short.
        if bytes.len() < Self::SIZE_OF_CWD_LEN {
            return Err(Error::new(ErrorCode::InvalidMessage, "message is too short"));
        }

        // Deserialize `cwd.len` field.
        let cwd_len: u32 = u32::from_le_bytes(
            bytes[Self::OFFSET_OF_CWD_LEN..Self::OFFSET_OF_CWD_LEN + Self::SIZE_OF_CWD_LEN]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid cwd.len"))?,
        );

        // Check if message is too short.
        if bytes.len() < Self::OFFSET_OF_CWD + cwd_len as usize {
            return Err(Error::new(ErrorCode::InvalidMessage, "message is too short"));
        }

        // Deserialize `cwd` field.
        let cwd: String = String::from_utf8(
            bytes[Self::OFFSET_OF_CWD..Self::OFFSET_OF_CWD + cwd_len as usize].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid message"))?;

        Ok(Self { cwd })
    }
}

impl MessagePartitioner for GetCurrentWorkingDirectoryResponse {
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_response(
            tid,
            LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryResponsePart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}
