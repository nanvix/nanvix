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
use ::core::{
    fmt::Debug,
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
    pm::ThreadIdentifier,
};
use sysapi::limits::PATH_MAX;

//==================================================================================================
// ChdirRequest
//==================================================================================================

#[derive(Debug)]
pub struct ChangeDirectoryRequest {
    /// Path
    pub path: String,
}

impl ChangeDirectoryRequest {
    /// Size of 'path length' field.
    pub const SIZE_OF_PATH_LENGTH: usize = mem::size_of::<u32>();
    /// Offset of 'path length' field.
    pub const OFFSET_OF_PATH_LENGTH: usize = 0;
    /// Offset of 'path' field.
    pub const OFFSET_OF_PATH: usize = Self::OFFSET_OF_PATH_LENGTH + Self::SIZE_OF_PATH_LENGTH;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::SIZE_OF_PATH_LENGTH + PATH_MAX;

    ///
    /// # Description
    ///
    /// Creates a request message for the `chdir()` system call.
    ///
    /// # Parameters
    ///
    /// - `path`: Path.
    ///
    /// # Returns
    ///
    /// Upon success, the request message for the `chdir()` system call is returned. Upon failure,
    /// an error is returned instead.
    ///
    pub fn new(path: &str) -> Result<Self, Error> {
        // Check if path is too long.
        if path.len() > PATH_MAX {
            #[cfg(target_os = "none")]
            ::syslog::error!("new(): path too long (path.len={:?})", path.len());
            return Err(Error::new(ErrorCode::InvalidArgument, "path too long"));
        }

        Ok(Self {
            path: path.to_string(),
        })
    }
}

impl MessageSerializer for ChangeDirectoryRequest {
    ///
    /// # Description
    ///
    /// Serializes a request message for the `chdir()` system call.
    ///
    /// # Returns
    ///
    /// A vector containing the serialized message is returned.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();

        let path_bytes: &[u8] = self.path.as_bytes();
        buffer.extend_from_slice(&(path_bytes.len() as u32).to_ne_bytes());
        buffer.extend_from_slice(path_bytes);

        buffer
    }
}

impl MessageDeserializer for ChangeDirectoryRequest {
    ///
    /// # Description
    ///
    /// Deserializes a request message for the `chdir()` system call.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Bytes to deserialize.
    ///
    /// # Returns
    ///
    /// Upon success, the deserialized request message for the `chdir()` system call is returned.
    ///
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if message is too short.
        if bytes.len() < Self::OFFSET_OF_PATH {
            #[cfg(target_os = "none")]
            ::syslog::error!("try_from_bytes(): message too short (bytes.len={:?})", bytes.len());
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Extract path length.
        let path_length: usize = u32::from_ne_bytes(
            bytes[Self::OFFSET_OF_PATH_LENGTH..Self::OFFSET_OF_PATH]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path length"))?,
        ) as usize;

        // Check if message is too short.
        if bytes.len() < Self::OFFSET_OF_PATH + path_length {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Extract the 'path' field.
        let path: String = String::from_utf8(
            bytes[Self::OFFSET_OF_PATH..Self::OFFSET_OF_PATH + path_length].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path"))?;

        Ok(Self { path })
    }
}

impl MessagePartitioner for ChangeDirectoryRequest {
    ///
    /// # Description
    ///
    /// Partitions a request message for the `chdir()` system call.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    /// - `total_parts`: Total number of parts.
    /// - `part_number`: Partition number.
    /// - `payload_size`: Size of the payload.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the partitioned message is returned. Upon failure, an error is returned instead.
    ///
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_request(
            tid,
            LinuxDaemonMessageHeader::ChangeDirectoryRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// ChdirResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ChangeDirectoryResponse {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ChangeDirectoryResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ChangeDirectoryResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier) -> Message {
        let message: ChangeDirectoryResponse = ChangeDirectoryResponse::new();
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::ChangeDirectoryResponse,
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
