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
use ::sysapi::{
    ffi::c_int,
    limits::PATH_MAX,
    sys_types::mode_t,
};

//==================================================================================================
// FileChmodAtRequest
//==================================================================================================

#[derive(Debug)]
pub struct FileChmodAtRequest {
    /// Directory file descriptor.
    pub dirfd: c_int,
    /// Mode.
    pub mode: mode_t,
    /// Flag.
    pub flag: c_int,
    /// Path.
    pub path: String,
}

impl FileChmodAtRequest {
    /// Size of 'directory file descriptor' field.
    pub const SIZE_OF_DIRFD: usize = mem::size_of::<c_int>();
    /// Size of 'mode' field.
    pub const SIZE_OF_MODE: usize = mem::size_of::<mode_t>();
    /// Size of 'flag' field.
    pub const SIZE_OF_FLAG: usize = mem::size_of::<c_int>();
    /// Size of 'path length' field.
    pub const SIZE_OF_PATH_LENGTH: usize = mem::size_of::<u32>();
    /// Offset of 'directory file descriptor' field.
    pub const OFFSET_OF_DIRFD: usize = 0;
    /// Offset of 'mode' field.
    pub const OFFSET_OF_MODE: usize = Self::OFFSET_OF_DIRFD + Self::SIZE_OF_DIRFD;
    /// Offset of 'flag' field.
    pub const OFFSET_OF_FLAG: usize = Self::OFFSET_OF_MODE + Self::SIZE_OF_MODE;
    /// Offset of 'path length' field.
    pub const OFFSET_OF_PATH_LENGTH: usize = Self::OFFSET_OF_FLAG + Self::SIZE_OF_FLAG;
    /// Offset of 'path' field.
    pub const OFFSET_OF_PATH: usize = Self::OFFSET_OF_PATH_LENGTH + Self::SIZE_OF_PATH_LENGTH;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::SIZE_OF_DIRFD
        + Self::SIZE_OF_MODE
        + Self::SIZE_OF_FLAG
        + Self::SIZE_OF_PATH_LENGTH
        + PATH_MAX;

    ///
    /// # Description
    ///
    /// Creates a request message for the `fchmodat()` system call.
    ///
    /// # Parameters
    ///
    /// - `dirfd`: Directory file descriptor.
    /// - `mode`: Mode.
    /// - `flag`: Flags.
    /// - `path`: Path.
    ///
    /// # Returns
    ///
    /// Upon success, the request message for the `fchmodat()` system call is returned. Upon failure,
    /// an error is returned instead.
    ///
    pub fn new(dirfd: c_int, mode: mode_t, flag: c_int, path: &str) -> Result<Self, Error> {
        // Check if path is too long.
        if path.len() > PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "old path too long"));
        }

        Ok(Self {
            dirfd,
            mode,
            flag,
            path: path.to_string(),
        })
    }
}

impl MessageSerializer for FileChmodAtRequest {
    ///
    /// # Description
    ///
    /// Serializes a request message for the `fchmodat()` system call.
    ///
    /// # Returns
    ///
    /// A vector containing the serialized message is returned.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();

        buffer.extend_from_slice(&self.dirfd.to_ne_bytes());
        buffer.extend_from_slice(&self.mode.to_ne_bytes());
        buffer.extend_from_slice(&self.flag.to_ne_bytes());
        let path_bytes: &[u8] = self.path.as_bytes();
        buffer.extend_from_slice(&(path_bytes.len() as u32).to_ne_bytes());
        buffer.extend_from_slice(path_bytes);

        buffer
    }
}

impl MessageDeserializer for FileChmodAtRequest {
    ///
    /// # Description
    ///
    /// Deserializes a request message for the `fchmodat()` system call.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Bytes to deserialize.
    ///
    /// # Returns
    ///
    /// Upon success, the deserialized request message for the `fchmodat()` system call is returned.
    ///
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if message is too short.
        if bytes.len() < Self::OFFSET_OF_PATH {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Extract the 'directory file descriptor' field.
        let dirfd: c_int = c_int::from_ne_bytes(
            bytes[Self::OFFSET_OF_DIRFD..Self::OFFSET_OF_MODE]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidMessage, "invalid directory file descriptor")
                })?,
        );

        // Extract the 'mode' field.
        let mode: mode_t = mode_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_MODE..Self::OFFSET_OF_FLAG]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid mode"))?,
        );

        // Extract the 'flag' field.
        let flag: c_int = c_int::from_ne_bytes(
            bytes[Self::OFFSET_OF_FLAG..Self::OFFSET_OF_PATH_LENGTH]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid flag"))?,
        );

        // Extract the 'path length' field.
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

        Ok(Self {
            dirfd,
            mode,
            flag,
            path,
        })
    }
}

impl MessagePartitioner for FileChmodAtRequest {
    ///
    /// # Description
    ///
    /// Partitions a request message for the `fchmodat()` system call.
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
            LinuxDaemonMessageHeader::FileChmodAtRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// FileChmodAtResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileChmodAtResponse {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileChmodAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileChmodAtResponse {
    /// Size of padding.
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
        let message: FileChmodAtResponse = FileChmodAtResponse::new();
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileChmodAtResponse,
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
