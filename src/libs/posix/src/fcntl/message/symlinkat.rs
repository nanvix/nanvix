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
// SymbolicLinkAtRequest
//==================================================================================================

///
/// # Description
///
/// This struct represents the request message of the `symlinkat()` system call.
///
#[derive(Debug)]
pub struct SymbolicLinkAtRequest {
    /// Directory file descriptor.
    pub dirfd: i32,
    /// Path.
    pub target: String,
    /// Symbolic link path.
    pub linkpath: String,
}

impl SymbolicLinkAtRequest {
    /// Sizes of 'directory file descriptor' field.
    const SIZE_OF_DIRFD: usize = mem::size_of::<i32>();
    /// Sizes of 'target length' field.
    const SIZE_OF_TARGET: usize = mem::size_of::<u32>();
    /// Sizes of 'link path length' field.
    const SIZE_OF_LINKPATH: usize = mem::size_of::<u32>();
    /// Offset of 'dirfd' field.
    const OFFSET_DIRFD: usize = 0;
    /// Offset of 'target length' field.
    const OFFSET_TARGET_LENGTH: usize = Self::OFFSET_DIRFD + Self::SIZE_OF_DIRFD;
    /// Offset of 'link path length' field.
    const OFFSET_LINKPATH_LENGTH: usize = Self::OFFSET_TARGET_LENGTH + Self::SIZE_OF_TARGET;
    /// Offset of 'target' field.
    const OFFSET_TARGET: usize = Self::OFFSET_LINKPATH_LENGTH + Self::SIZE_OF_LINKPATH;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::SIZE_OF_DIRFD
        + Self::SIZE_OF_TARGET
        + Self::SIZE_OF_LINKPATH
        + limits::PATH_MAX
        + limits::PATH_MAX;

    ///
    /// # Description
    ///
    /// Creates a new request message for the `symlinkat()` system call.
    ///
    pub fn new(target: String, dirfd: i32, linkpath: String) -> Result<Self, Error> {
        // Check if the target is too long.
        if target.len() > limits::PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "target too long"));
        }

        // Check if the link path is too long.
        if linkpath.len() > limits::PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "link path too long"));
        }

        Ok(SymbolicLinkAtRequest {
            dirfd,
            target,
            linkpath,
        })
    }
}

impl MessageSerializer for SymbolicLinkAtRequest {
    ///
    /// # Description
    ///
    /// Serializes the request message of the `symlinkat()` system call.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();

        // Serialize the 'directory file descriptor' field.
        buffer.extend_from_slice(&self.dirfd.to_ne_bytes());
        // Serialize the 'target length' field.
        buffer.extend_from_slice(&(self.target.len() as u32).to_ne_bytes());
        // Serialize the 'link path length' field.
        buffer.extend_from_slice(&(self.linkpath.len() as u32).to_ne_bytes());
        // Serialize the 'target' field.
        buffer.extend_from_slice(self.target.as_bytes());
        // Serialize the 'link path' field.
        buffer.extend_from_slice(self.linkpath.as_bytes());

        buffer
    }
}

impl MessageDeserializer for SymbolicLinkAtRequest {
    ///
    /// # Description
    ///
    /// Deserializes the request message of the `symlinkat()` system call.
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
        if bytes.len() < SymbolicLinkAtRequest::OFFSET_TARGET {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Check if message is too long.
        if bytes.len() > SymbolicLinkAtRequest::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too long"));
        }

        // Deserialize the 'directory file descriptor' field.
        let dirfd: i32 = i32::from_ne_bytes(
            bytes[SymbolicLinkAtRequest::OFFSET_DIRFD..SymbolicLinkAtRequest::OFFSET_TARGET_LENGTH]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid dirfd"))?,
        );
        // Deserialize the 'target length' field.
        let target_length: usize = u32::from_ne_bytes(
            bytes[SymbolicLinkAtRequest::OFFSET_TARGET_LENGTH
                ..SymbolicLinkAtRequest::OFFSET_LINKPATH_LENGTH]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid target length"))?,
        ) as usize;
        // Deserialize the 'link path length' field.
        let linkpath_length: usize = u32::from_ne_bytes(
            bytes[SymbolicLinkAtRequest::OFFSET_LINKPATH_LENGTH
                ..SymbolicLinkAtRequest::OFFSET_TARGET]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid link path length"))?,
        ) as usize;

        // Check if message is too short.
        if bytes.len() < SymbolicLinkAtRequest::OFFSET_TARGET + target_length + linkpath_length {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Deserialize the 'target' field.
        let target: String = String::from_utf8(
            bytes[SymbolicLinkAtRequest::OFFSET_TARGET
                ..SymbolicLinkAtRequest::OFFSET_TARGET + target_length]
                .to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid target"))?;
        // Deserialize the 'link path' field.
        let linkpath: String = String::from_utf8(
            bytes[SymbolicLinkAtRequest::OFFSET_TARGET + target_length
                ..SymbolicLinkAtRequest::OFFSET_TARGET + target_length + linkpath_length]
                .to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid link path"))?;

        Ok(SymbolicLinkAtRequest {
            dirfd,
            target,
            linkpath,
        })
    }
}

impl MessagePartitioner for SymbolicLinkAtRequest {
    ///
    /// # Description
    ///
    /// Creates a new message partition for the `symlinkat()` system call.
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
            LinuxDaemonMessageHeader::SymbolicLinkAtRequestPart,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// SymbolicLinkAtResponse
//==================================================================================================

#[repr(C, packed)]
pub struct SymbolicLinkAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(SymbolicLinkAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl SymbolicLinkAtResponse {
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
        let message: SymbolicLinkAtResponse = SymbolicLinkAtResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::SymbolicLinkAtResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());
        message
    }
}
