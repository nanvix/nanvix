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
    sys::stat::stat,
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
// FileStatRequest
//==================================================================================================

///
/// # Description
///
/// This struct represents the request message of the `fstatat()` system call.
///
#[derive(Debug)]
pub struct FileStatRequest {
    /// Directory file descriptor.
    pub dirfd: i32,
    /// Flags.
    pub flag: i32,
    /// Path.
    pub path: String,
}

impl FileStatRequest {
    /// Sizes of 'directory file descriptor' field.
    const SIZE_OF_DIRFD: usize = mem::size_of::<i32>();
    /// Sizes of 'flags' field.
    const SIZE_OF_FLAG: usize = mem::size_of::<i32>();
    /// Sizes of 'path length' field.
    const SIZE_OF_PATH_LENGTH: usize = mem::size_of::<u32>();
    /// Offsets to 'directory file descriptor' field.
    const OFFSET_OF_DIRFD: usize = 0;
    /// Offsets to 'flags' field.
    const OFFSET_OF_FLAG: usize = Self::OFFSET_OF_DIRFD + Self::SIZE_OF_DIRFD;
    /// Offsets to 'path length' field.
    const OFFSET_OF_PATH_LENGTH: usize = Self::OFFSET_OF_FLAG + Self::SIZE_OF_FLAG;
    /// Offsets to 'path' field.
    const OFFSET_OF_PATH: usize = Self::OFFSET_OF_PATH_LENGTH + Self::SIZE_OF_PATH_LENGTH;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize =
        Self::SIZE_OF_DIRFD + Self::SIZE_OF_FLAG + Self::SIZE_OF_PATH_LENGTH + limits::PATH_MAX;

    ///
    /// # Description
    ///
    /// Creates a new request message for the `fstatat()` system call.
    ///
    pub fn new(dirfd: i32, path: String, flag: i32) -> Self {
        FileStatRequest { dirfd, flag, path }
    }
}

impl MessageSerializer for FileStatRequest {
    ///
    /// # Description
    ///
    /// Serializes the request message of the `fstatat()` system call.
    ///
    /// # Returns
    ///
    /// The serialized message.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::with_capacity(Self::OFFSET_OF_PATH + self.path.len());

        // Serialize directory file descriptor.
        bytes.extend_from_slice(&self.dirfd.to_le_bytes());
        // Serialize flags.
        bytes.extend_from_slice(&self.flag.to_le_bytes());
        // Serialize path length.
        bytes.extend_from_slice(&(self.path.len() as u32).to_le_bytes());
        // Serialize path.
        bytes.extend_from_slice(self.path.as_bytes());

        bytes
    }
}

impl MessageDeserializer for FileStatRequest {
    ///
    /// # Description
    ///
    /// Deserializes the request message of the `fstatat()` system call.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Bytes to deserialize.
    ///
    /// # Returns
    ///
    /// Upon success, the deserialized message is returned. Upon failure, an error is returned.
    ///
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if message is too short.
        if bytes.len() < Self::OFFSET_OF_PATH {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Deserialize directory file descriptor.
        let dirfd: i32 = i32::from_le_bytes(
            bytes[Self::OFFSET_OF_DIRFD..Self::OFFSET_OF_FLAG]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid dirfd"))?,
        );
        // Deserialize flags.
        let flag: i32 = i32::from_le_bytes(
            bytes[Self::OFFSET_OF_FLAG..Self::OFFSET_OF_PATH_LENGTH]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid flag"))?,
        );
        // Deserialize path length.
        let path_len: usize = u32::from_le_bytes(
            bytes[Self::OFFSET_OF_PATH_LENGTH..Self::OFFSET_OF_PATH]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path length"))?,
        ) as usize;

        // Check if message is too short.
        if bytes.len() < Self::OFFSET_OF_PATH + path_len {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Deserialize path.
        let path: String = String::from_utf8(
            bytes[Self::OFFSET_OF_PATH..Self::OFFSET_OF_PATH + path_len].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path"))?;

        Ok(FileStatRequest { dirfd, flag, path })
    }
}

impl MessagePartitioner for FileStatRequest {
    ///
    /// # Description
    ///
    /// Creates a new message partition for the `fstatat()` system call.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `part_number`: Partition number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the new message partition is returned. Upon failure, an error is returned.
    ///
    fn new_part(
        pid: ProcessIdentifier,
        part_number: u32,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_request(
            pid,
            LinuxDaemonMessageHeader::FileStatRequestPart,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// FileStatResponse
//==================================================================================================

///
/// # Description
///
/// This struct represents the response message of the `fstatat()` system call.
///
#[derive(Debug)]
pub struct FileStatResponse {
    /// File status.
    pub stat: stat,
}

impl FileStatResponse {
    /// Size of file status field.
    const SIZE_OF_STAT: usize = mem::size_of::<stat>();

    ///
    /// # Description
    ///
    /// Creates a new response message for the `fstatat()` system call.
    ///
    pub fn new(stat: stat) -> Self {
        FileStatResponse { stat }
    }
}

impl MessageSerializer for FileStatResponse {
    ///
    /// # Description
    ///
    /// Serializes the response message of the `fstatat()` system call.
    ///
    /// # Returns
    ///
    /// The serialized message.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::with_capacity(Self::SIZE_OF_STAT);

        // Serialize file status.
        bytes.extend_from_slice(self.stat.to_bytes().as_slice());

        bytes
    }
}

impl MessageDeserializer for FileStatResponse {
    ///
    /// # Description
    ///
    /// Deserializes the response message of the `fstatat()` system call.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Bytes to deserialize.
    ///
    /// # Returns
    ///
    /// Upon success, the deserialized message is returned. Upon failure, an error is returned.
    ///
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if message is too short.
        if bytes.len() < Self::SIZE_OF_STAT {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        Ok(FileStatResponse {
            stat: stat::try_from_bytes(bytes)?,
        })
    }
}

impl MessagePartitioner for FileStatResponse {
    ///
    /// # Description
    ///
    /// Creates a new message partition for the `fstatat()` system call.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `part_number`: Partition number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the new message partition is returned. Upon failure, an error is returned.
    ///
    fn new_part(
        pid: ProcessIdentifier,
        part_number: u32,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_response(
            pid,
            LinuxDaemonMessageHeader::FileStatResponsePart,
            part_number,
            payload_size,
            payload,
        )
    }
}
