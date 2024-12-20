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
    sys::types::mode_t,
    LinuxDaemonMessage,
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
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// MakeDirectoryAtRequest
//==================================================================================================

///
/// # Description
///
/// This struct represents the request message of the `mkdirat()` system call.
///
#[derive(Debug)]
pub struct MakeDirectoryAtRequest {
    /// Directory file descriptor.
    pub dirfd: i32,
    /// Path.
    pub pathname: String,
    /// Mode.
    pub mode: mode_t,
}

impl MakeDirectoryAtRequest {
    /// Sizes of 'directory file descriptor' field.
    const SIZE_OF_DIRFD: usize = mem::size_of::<i32>();
    /// Sizes of 'path length' field.
    const SIZE_OF_PATH_LENGTH: usize = mem::size_of::<u32>();
    /// Sizes of 'mode' field.
    const SIZE_OF_MODE: usize = mem::size_of::<mode_t>();
    /// Offset of 'directory file descriptor' field.
    const OFFSET_OF_DIRFD: usize = 0;
    /// Offset of 'path length' field.
    const OFFSET_OF_PATH_LENGTH: usize = Self::OFFSET_OF_DIRFD + Self::SIZE_OF_DIRFD;
    /// Offset of 'mode' field.
    const OFFSET_OF_MODE: usize = Self::OFFSET_OF_PATH_LENGTH + Self::SIZE_OF_PATH_LENGTH;
    /// Offset of 'path' field.
    const OFFSET_OF_PATH: usize = Self::OFFSET_OF_MODE + Self::SIZE_OF_MODE;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize =
        Self::SIZE_OF_DIRFD + Self::SIZE_OF_PATH_LENGTH + Self::SIZE_OF_MODE + limits::PATH_MAX;

    ///
    /// # Description
    ///
    /// Creates a new request message for the `mkdirat()` system call.
    ///
    /// # Parameters
    ///
    /// - `dirfd`: Directory file descriptor.
    /// - `pathname`: Path.
    /// - `mode`: Mode.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the request message for the `mkdirat()` system call.
    ///
    pub fn new(dirfd: i32, pathname: String, mode: mode_t) -> Result<Self, Error> {
        // Check if the path is too long.
        if pathname.len() > limits::PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "path too long"));
        }

        Ok(Self {
            dirfd,
            pathname,
            mode,
        })
    }
}

impl MessageSerializer for MakeDirectoryAtRequest {
    ///
    /// # Description
    ///
    /// Serializes a request message for the `mkdirat()` system call.
    ///
    /// # Parameters
    ///
    /// - `self`: Request message.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the serialized message.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        // Allocate buffer.
        let mut buffer: Vec<u8> = Vec::new();

        // Serialize directory file descriptor.
        buffer.extend_from_slice(&self.dirfd.to_ne_bytes());
        // Serialize path length.
        buffer.extend_from_slice(&(self.pathname.len() as u32).to_ne_bytes());
        // Serialize mode.
        buffer.extend_from_slice(&self.mode.to_ne_bytes());
        // Serialize path.
        buffer.extend_from_slice(self.pathname.as_bytes());

        buffer
    }
}

impl MessageDeserializer for MakeDirectoryAtRequest {
    ///
    /// # Description
    ///
    /// Deserializes a request message for the `mkdirat()` system call.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Bytes to deserialize.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the deserialized message. Upon failure, an error is
    /// returned.
    ///
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the message is too short.
        if bytes.len() < MakeDirectoryAtRequest::OFFSET_OF_PATH {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Check if the message is too long.
        if bytes.len() > MakeDirectoryAtRequest::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too long"));
        }

        // Deserialize directory file descriptor.
        let dirfd: i32 = i32::from_ne_bytes(
            bytes[MakeDirectoryAtRequest::OFFSET_OF_DIRFD
                ..MakeDirectoryAtRequest::OFFSET_OF_PATH_LENGTH]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid dirfd"))?,
        );
        // Deserialize path length.
        let path_length: usize = u32::from_ne_bytes(
            bytes[MakeDirectoryAtRequest::OFFSET_OF_PATH_LENGTH
                ..MakeDirectoryAtRequest::OFFSET_OF_MODE]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path length"))?,
        ) as usize;
        // Deserialize mode.
        let mode: mode_t = mode_t::from_ne_bytes(
            bytes[MakeDirectoryAtRequest::OFFSET_OF_MODE..MakeDirectoryAtRequest::OFFSET_OF_PATH]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid mode"))?,
        );

        // Check if message is too short.
        if bytes.len() < MakeDirectoryAtRequest::OFFSET_OF_PATH + path_length {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Check if path is too long.
        if path_length > limits::PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "path too long"));
        }

        // Deserialize path.
        let pathname: String = String::from_utf8(
            bytes[MakeDirectoryAtRequest::OFFSET_OF_PATH
                ..MakeDirectoryAtRequest::OFFSET_OF_PATH + path_length]
                .to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path"))?;

        Ok(Self {
            dirfd,
            pathname,
            mode,
        })
    }
}

impl MessagePartitioner for MakeDirectoryAtRequest {
    ///
    /// # Description
    ///
    /// Creates a new message part for the `mkdirat()` system call.
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
            LinuxDaemonMessageHeader::MakeDirectoryAtRequestPart,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// MakeDirectoryAtResponse
//==================================================================================================

#[repr(C, packed)]
pub struct MakeDirectoryAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(MakeDirectoryAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl MakeDirectoryAtResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(ret: i32) -> Self {
        Self {
            ret,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, ret: i32) -> Message {
        let message: MakeDirectoryAtResponse = MakeDirectoryAtResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::MakeDirectoryAtResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());
        message
    }
}
