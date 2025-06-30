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
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use sysapi::{
    limits::PATH_MAX,
    sys_stat::{
        stat,
        StatError,
    },
};

//==================================================================================================
// FileStatAtRequest
//==================================================================================================

///
/// # Description
///
/// This struct represents the request message of the `fstatat()` system call.
///
#[derive(Debug)]
pub struct FileStatAtRequest {
    /// Directory file descriptor.
    pub dirfd: i32,
    /// Flags.
    pub flag: i32,
    /// Path.
    pub path: String,
}

impl FileStatAtRequest {
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
        Self::SIZE_OF_DIRFD + Self::SIZE_OF_FLAG + Self::SIZE_OF_PATH_LENGTH + PATH_MAX;

    ///
    /// # Description
    ///
    /// Creates a new request message for the `fstatat()` system call.
    ///
    pub fn new(dirfd: i32, path: String, flag: i32) -> Result<Self, Error> {
        // Check if path is too long.
        if path.len() > PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "path too long"));
        }

        Ok(FileStatAtRequest { dirfd, flag, path })
    }
}

impl MessageSerializer for FileStatAtRequest {
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
        let path_bytes: &[u8] = self.path.as_bytes();
        // Serialize path length.
        bytes.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        // Serialize path.
        bytes.extend_from_slice(path_bytes);

        bytes
    }
}

impl MessageDeserializer for FileStatAtRequest {
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

        // Check if message is too long.
        if bytes.len() > Self::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too long"));
        }

        // Deserialize directory file descriptor.
        let dirfd: i32 = i32::from_le_bytes(
            bytes[Self::OFFSET_OF_DIRFD..(Self::OFFSET_OF_DIRFD + Self::SIZE_OF_DIRFD)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid dirfd"))?,
        );
        // Deserialize flags.
        let flag: i32 = i32::from_le_bytes(
            bytes[Self::OFFSET_OF_FLAG..(Self::OFFSET_OF_FLAG + Self::SIZE_OF_FLAG)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid flag"))?,
        );
        // Deserialize path length.
        let path_len: usize = u32::from_le_bytes(
            bytes[Self::OFFSET_OF_PATH_LENGTH
                ..(Self::OFFSET_OF_PATH_LENGTH + Self::SIZE_OF_PATH_LENGTH)]
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

        Ok(FileStatAtRequest { dirfd, flag, path })
    }
}

impl MessagePartitioner for FileStatAtRequest {
    ///
    /// # Description
    ///
    /// Creates a new message partition for the `fstatat()` system call.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    /// - `total_parts`: Total number of parts.
    /// - `part_number`: Partition number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the new message partition is returned. Upon failure, an error is returned.
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
            LinuxDaemonMessageHeader::FileStatAtRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// FileStatAtResponse
//==================================================================================================

///
/// # Description
///
/// This struct represents the response message of the `fstatat()` system call.
///
#[derive(Debug)]
pub struct FileStatAtResponse {
    /// File status.
    pub stat: stat,
}

impl FileStatAtResponse {
    /// Size of file status field.
    const SIZE_OF_STAT: usize = mem::size_of::<stat>();

    ///
    /// # Description
    ///
    /// Creates a new response message for the `fstatat()` system call.
    ///
    pub fn new(stat: stat) -> Self {
        FileStatAtResponse { stat }
    }
}

impl MessageSerializer for FileStatAtResponse {
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

impl MessageDeserializer for FileStatAtResponse {
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

        Ok(FileStatAtResponse {
            stat: match stat::try_from_bytes(bytes) {
                Ok(stat) => stat,
                Err(StatError::InvalidSize) => {
                    return Err(Error::new(ErrorCode::InvalidMessage, "invalid file status size"))
                },
                Err(StatError::FailedToParseDev) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse dev in file status",
                    ))
                },
                Err(StatError::FailedToParseIno) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse inode in file status",
                    ))
                },
                Err(StatError::FailedToParseMode) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse mode in file status",
                    ))
                },
                Err(StatError::FailedToParseNlink) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse nlink in file status",
                    ))
                },
                Err(StatError::FailedToParseUid) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse uid in file status",
                    ))
                },
                Err(StatError::FailedToParseGid) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse gid in file status",
                    ))
                },
                Err(StatError::FailedToParseRdev) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse rdev in file status",
                    ))
                },
                Err(StatError::FailedToParseSize) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse size in file status",
                    ))
                },
                Err(StatError::FailedToParseAtim) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse atim in file status",
                    ))
                },
                Err(StatError::FailedToParseMtim) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse mtim in file status",
                    ))
                },
                Err(StatError::FailedToParseCtim) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse ctim in file status",
                    ))
                },
                Err(StatError::FailedToParseBlksize) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse blksize in file status",
                    ))
                },
                Err(StatError::FailedToParseBlocks) => {
                    return Err(Error::new(
                        ErrorCode::InvalidMessage,
                        "failed to parse blocks in file status",
                    ))
                },
            },
        })
    }
}

impl MessagePartitioner for FileStatAtResponse {
    ///
    /// # Description
    ///
    /// Creates a new message partition for the `fstatat()` system call.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    /// - `total_parts`: Total number of parts.
    /// - `part_number`: Partition number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the new message partition is returned. Upon failure, an error is returned.
    ///
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_response(
            tid,
            LinuxDaemonMessageHeader::FileStatAtResponsePart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}
