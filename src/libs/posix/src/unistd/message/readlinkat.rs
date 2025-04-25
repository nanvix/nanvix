// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    limits,
    message::{
        LinuxDaemonMessagePart,
        MessageDeserializer,
        MessagePartitioner,
        MessageSerializer,
    },
    LinuxDaemonMessageHeader,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::core::{
    convert::TryInto,
    mem,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// ReadLinkAtRequest
//==================================================================================================

///
/// # Description
///
/// This struct represents the request message of the `readlinkat()` system call.
///
#[derive(Debug)]
pub struct ReadLinkAtRequest {
    /// Directory file descriptor.
    pub dirfd: i32,
    /// Path.
    pub path: String,
    /// Buffer size.
    pub bufsiz: usize,
}

impl ReadLinkAtRequest {
    /// Sizes of 'directory file descriptor' field.
    const SIZE_OF_DIRFD: usize = mem::size_of::<i32>();
    /// Sizes of 'path length' field.
    const SIZE_OF_PATH_LENGTH: usize = mem::size_of::<u32>();
    /// Sizes of 'buffer size' field.
    const SIZE_OF_BUFSIZ: usize = mem::size_of::<u32>();
    /// Offset of 'directory file descriptor' field.
    const OFFSET_OF_DIRFD: usize = 0;
    /// Offset of 'path' field.
    const OFFSET_OF_PATH_LENGTH: usize = Self::OFFSET_OF_DIRFD + Self::SIZE_OF_DIRFD;
    /// Offset of 'buffer size' field.
    const OFFSET_OF_BUFSIZ: usize = Self::OFFSET_OF_PATH_LENGTH + Self::SIZE_OF_PATH_LENGTH;
    /// Offset of 'path' field.
    const OFFSET_OF_PATH: usize = Self::OFFSET_OF_BUFSIZ + Self::SIZE_OF_BUFSIZ;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize =
        Self::SIZE_OF_DIRFD + Self::SIZE_OF_PATH_LENGTH + Self::SIZE_OF_BUFSIZ + limits::PATH_MAX;

    ///
    /// # Description
    ///
    /// Creates a request message of the `readlinkat()` system call.
    ///
    /// # Parameters
    ///
    /// - `dirfd`: Directory file descriptor.
    /// - `path`: Path.
    /// - `bufsiz`: Buffer size.
    ///
    /// # Returns
    ///
    /// Upon success, the request message of the `readlinkat()` system call is returned. Upon failure,
    /// an error is returned instead.
    ///
    pub fn new(dirfd: i32, path: String, bufsiz: usize) -> Result<Self, Error> {
        // Check if the path is too long.
        if path.len() > limits::PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "old path too long"));
        }

        Ok(Self {
            dirfd,
            path,
            bufsiz,
        })
    }
}

impl MessageSerializer for ReadLinkAtRequest {
    ///
    /// # Description
    ///
    /// Serializes a request message of the `readlinkat()` system call.
    ///
    /// # Returns
    ///
    /// A vector containing the serialized message is returned.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();

        buffer.extend_from_slice(&self.dirfd.to_ne_bytes());
        buffer.extend_from_slice(&(self.path.len() as u32).to_ne_bytes());
        buffer.extend_from_slice(&(self.bufsiz as u32).to_ne_bytes());
        buffer.extend_from_slice(self.path.as_bytes());

        buffer
    }
}

impl MessageDeserializer for ReadLinkAtRequest {
    ///
    /// # Description
    ///
    /// Deserializes a request message of the `readlinkat()` system call.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Bytes to deserialize.
    ///
    /// # Returns
    ///
    /// Upon success, the deserialized request message of the `readlinkat()` system call is returned.
    ///
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the message is too short.
        if bytes.len() < Self::OFFSET_OF_PATH {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Extracts the directory file descriptor.
        let dirfd: i32 = i32::from_ne_bytes(
            bytes[Self::OFFSET_OF_DIRFD..(Self::OFFSET_OF_DIRFD + Self::SIZE_OF_DIRFD)]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidMessage, "invalid directory file descriptor")
                })?,
        );

        // Extracts the path length.
        let path_length: usize = u32::from_ne_bytes(
            bytes[Self::OFFSET_OF_PATH_LENGTH
                ..(Self::OFFSET_OF_PATH_LENGTH + Self::SIZE_OF_PATH_LENGTH)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path length"))?,
        ) as usize;

        // Extracts the buffer size.
        let bufsiz: u32 = u32::from_ne_bytes(
            bytes[Self::OFFSET_OF_BUFSIZ..(Self::OFFSET_OF_BUFSIZ + Self::SIZE_OF_BUFSIZ)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid buffer size"))?,
        );

        // Check if the message is too short.
        if bytes.len() < Self::OFFSET_OF_PATH + path_length {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Extracts the path.
        let path: String = String::from_utf8(
            bytes[Self::OFFSET_OF_PATH..(Self::OFFSET_OF_PATH + path_length)].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path"))?;

        Ok(Self {
            dirfd,
            path,
            bufsiz: bufsiz as usize,
        })
    }
}

impl MessagePartitioner for ReadLinkAtRequest {
    ///
    /// # Description
    ///
    /// Partitions a request message of the `readlinkat()` system call.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `part_number`: Partition number.
    /// - `payload_size`: Size of the payload.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the partitioned message is returned. Upon failure, an error is returned instead.
    ///
    fn new_part(
        pid: ProcessIdentifier,
        part_number: u32,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_request(
            pid,
            LinuxDaemonMessageHeader::ReadLinkAtRequestPart,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// ReadLinkAtResponse
//==================================================================================================

#[derive(Debug)]
pub struct ReadLinkAtResponse {
    /// Buffer.
    pub buffer: Vec<u8>,
}

impl ReadLinkAtResponse {
    /// Size of 'buffer length' field.
    const SIZE_OF_BUFFER_LENGTH: usize = mem::size_of::<u32>();
    /// Offset of 'buffer length' field.
    const OFFSET_OF_BUFFER_LENGTH: usize = 0;
    /// Offset of 'buffer ' field.
    const OFFSET_OF_BUFFER: usize = Self::OFFSET_OF_BUFFER_LENGTH + Self::SIZE_OF_BUFFER_LENGTH;

    /// Maximum size of buffer.
    // FIXME: this should be SSIZE_MAX.
    pub const BUFFER_SIZE_MAX: usize = nvx::sys::arch::mem::PAGE_SIZE / 2;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::SIZE_OF_BUFFER_LENGTH + Self::BUFFER_SIZE_MAX;

    ///
    /// # Description
    ///
    /// Creates a response message of the `readlinkat()` system call.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Buffer.
    ///
    /// # Returns
    ///
    /// Upon success, the response message of the `readlinkat()` system call is returned. Upon failure,
    /// an error is returned instead.
    ///
    pub fn new(buffer: Vec<u8>) -> Result<Self, Error> {
        // Check if buffer has a valid size.
        if buffer.is_empty() {
            return Err(Error::new(ErrorCode::InvalidMessage, "buffer too short"));
        }
        // Check if the buffer is too long.
        if buffer.len() > limits::SSIZE_MAX as usize {
            return Err(Error::new(ErrorCode::InvalidMessage, "buffer too long"));
        }

        Ok(Self { buffer })
    }
}

impl MessageSerializer for ReadLinkAtResponse {
    ///
    /// # Description
    ///
    /// Serializes a response message of the `readlinkat()` system call.
    ///
    /// # Returns
    ///
    /// A vector containing the serialized message is returned.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();

        buffer.extend_from_slice(&(self.buffer.len() as u32).to_ne_bytes());
        buffer.extend_from_slice(&self.buffer);

        buffer
    }
}

impl MessageDeserializer for ReadLinkAtResponse {
    ///
    /// # Description
    ///
    /// Deserializes a response message of the `readlinkat()` system call.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Bytes to deserialize.
    ///
    /// # Returns
    ///
    /// Upon success, the deserialized response message of the `readlinkat()` system call is returned.
    ///
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the message is too short.
        if bytes.len() < Self::OFFSET_OF_BUFFER {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Extracts the buffer length.
        let buffer_length: usize = u32::from_ne_bytes(
            bytes[Self::OFFSET_OF_BUFFER_LENGTH
                ..(Self::OFFSET_OF_BUFFER_LENGTH + Self::SIZE_OF_BUFFER_LENGTH)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid buffer length"))?,
        ) as usize;

        // Check if the message is too short.
        if bytes.len() < Self::OFFSET_OF_BUFFER + buffer_length {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Extracts the buffer.
        let buffer: Vec<u8> =
            bytes[Self::OFFSET_OF_BUFFER..(Self::OFFSET_OF_BUFFER + buffer_length)].to_vec();

        Ok(Self { buffer })
    }
}

impl MessagePartitioner for ReadLinkAtResponse {
    ///
    /// # Description
    ///
    /// Partitions a response message of the `readlinkat()` system call.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `part_number`: Partition number.
    /// - `payload_size`: Size of the payload.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the partitioned message is returned. Upon failure, an error is returned instead.
    ///
    fn new_part(
        pid: ProcessIdentifier,
        part_number: u32,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_response(
            pid,
            LinuxDaemonMessageHeader::ReadLinkAtResponsePart,
            part_number,
            payload_size,
            payload,
        )
    }
}
