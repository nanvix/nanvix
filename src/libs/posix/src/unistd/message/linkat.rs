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
// LinkAtRequest
//==================================================================================================

///
/// # Description
///
/// This struct represents the request message of the `linkat()` system call.
///
#[derive(Debug)]
pub struct LinkAtRequest {
    /// Directory file descriptor.
    pub olddirfd: i32,
    /// Old path.
    pub oldpath: String,
    /// New directory file descriptor.
    pub newdirfd: i32,
    /// New path.
    pub newpath: String,
    /// Flags.
    pub flags: i32,
}

impl LinkAtRequest {
    /// Sizes of 'old directory file descriptor' field.
    const SIZE_OF_OLDDIRFD: usize = mem::size_of::<i32>();
    /// Sizes of 'old path length' field.
    const SIZE_OF_OLDPATH_LENGTH: usize = mem::size_of::<u32>();
    /// Sizes of 'new directory file descriptor' field.
    const SIZE_OF_NEWDIRFD: usize = mem::size_of::<i32>();
    /// Sizes of 'new path length' field.
    const SIZE_OF_NEWPATH_LENGTH: usize = mem::size_of::<u32>();
    /// Sizes of 'flags' field.
    const SIZE_OF_FLAGS: usize = mem::size_of::<i32>();
    /// Offset of 'old directory file descriptor' field.
    const OFFSET_OLDDIRFD: usize = 0;
    /// Offset of 'old path length' field.
    const OFFSET_OLDPATH_LENGTH: usize = Self::OFFSET_OLDDIRFD + Self::SIZE_OF_OLDDIRFD;
    /// Offset of 'new directory file descriptor' field.
    const OFFSET_NEWDIRFD: usize = Self::OFFSET_OLDPATH_LENGTH + Self::SIZE_OF_OLDPATH_LENGTH;
    /// Offset of 'new path length' field.
    const OFFSET_NEWPATH_LENGTH: usize = Self::OFFSET_NEWDIRFD + Self::SIZE_OF_NEWDIRFD;
    /// Offset of 'flags' field.
    const OFFSET_FLAGS: usize = Self::OFFSET_NEWPATH_LENGTH + Self::SIZE_OF_NEWPATH_LENGTH;
    /// Offset of 'old path' field.
    const OFFSET_OLDPATH: usize = Self::OFFSET_FLAGS + Self::SIZE_OF_FLAGS;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::SIZE_OF_OLDDIRFD
        + Self::SIZE_OF_OLDPATH_LENGTH
        + limits::PATH_MAX
        + Self::SIZE_OF_NEWDIRFD
        + Self::SIZE_OF_NEWPATH_LENGTH
        + limits::PATH_MAX
        + Self::SIZE_OF_FLAGS;

    ///
    /// # Description
    ///
    /// Creates a new request message for the `linkat()` system call.
    ///
    pub fn new(
        olddirfd: i32,
        oldpath: String,
        newdirfd: i32,
        newpath: String,
        flags: i32,
    ) -> Result<Self, Error> {
        // Check if the old path is too long.
        if oldpath.len() > limits::PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "old path too long"));
        }

        // Check if the new path is too long.
        if newpath.len() > limits::PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "new path too long"));
        }

        Ok(Self {
            olddirfd,
            oldpath,
            newdirfd,
            newpath,
            flags,
        })
    }
}

impl MessageSerializer for LinkAtRequest {
    ///
    /// # Description
    ///
    /// Serializes a request message for the `linkat()` system call.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        // Serialize 'old directory file descriptor' field.
        bytes.extend_from_slice(&self.olddirfd.to_ne_bytes());
        // Serialize 'old path length' field.
        bytes.extend_from_slice(&(self.oldpath.len() as u32).to_ne_bytes());
        // Serialize 'new directory file descriptor' field.
        bytes.extend_from_slice(&self.newdirfd.to_ne_bytes());
        // Serialize 'new path length' field.
        bytes.extend_from_slice(&(self.newpath.len() as u32).to_ne_bytes());
        // Serialize 'flags' field.
        bytes.extend_from_slice(&self.flags.to_ne_bytes());
        // Serialize 'old path' field.
        bytes.extend_from_slice(self.oldpath.as_bytes());
        // Serialize 'new path' field.
        bytes.extend_from_slice(self.newpath.as_bytes());

        bytes
    }
}

impl MessageDeserializer for LinkAtRequest {
    ///
    /// # Description
    ///
    /// Deserializes a request message for the `linkat()` system call.
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
        // Check if the message is too short.
        if bytes.len() < LinkAtRequest::OFFSET_OLDPATH {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Check if message is too long.
        if bytes.len() > LinkAtRequest::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too long"));
        }

        // Deserialize 'old directory file descriptor' field.
        let olddirfd: i32 = i32::from_ne_bytes(
            bytes[LinkAtRequest::OFFSET_OLDDIRFD
                ..(LinkAtRequest::OFFSET_OLDDIRFD + LinkAtRequest::SIZE_OF_OLDDIRFD)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid olddirfd"))?,
        );
        // Deserialize 'old path length' field.
        let oldpath_length: usize = u32::from_ne_bytes(
            bytes[LinkAtRequest::OFFSET_OLDPATH_LENGTH
                ..(LinkAtRequest::OFFSET_OLDPATH_LENGTH + LinkAtRequest::SIZE_OF_OLDPATH_LENGTH)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid oldpath length"))?,
        ) as usize;
        // Deserialize 'new directory file descriptor' field.
        let newdirfd: i32 = i32::from_ne_bytes(
            bytes[LinkAtRequest::OFFSET_NEWDIRFD
                ..(LinkAtRequest::OFFSET_NEWDIRFD + LinkAtRequest::SIZE_OF_NEWDIRFD)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid newdirfd"))?,
        );
        // Deserialize 'new path length' field.
        let newpath_length: usize = u32::from_ne_bytes(
            bytes[LinkAtRequest::OFFSET_NEWPATH_LENGTH
                ..(LinkAtRequest::OFFSET_NEWPATH_LENGTH + LinkAtRequest::SIZE_OF_NEWPATH_LENGTH)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid newpath length"))?,
        ) as usize;
        // Deserialize 'flags' field.
        let flags: i32 = i32::from_ne_bytes(
            bytes[LinkAtRequest::OFFSET_FLAGS
                ..(LinkAtRequest::OFFSET_FLAGS + LinkAtRequest::SIZE_OF_FLAGS)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid flags"))?,
        );

        // Check if the message is too short.
        if bytes.len() < LinkAtRequest::OFFSET_OLDPATH + oldpath_length + newpath_length {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Deserialize 'old path' field.
        let oldpath: String = String::from_utf8(
            bytes[LinkAtRequest::OFFSET_OLDPATH..(LinkAtRequest::OFFSET_OLDPATH + oldpath_length)]
                .to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid old path"))?;

        // Deserialize 'new path' field.
        let newpath: String = String::from_utf8(
            bytes[(LinkAtRequest::OFFSET_OLDPATH + oldpath_length)
                ..(LinkAtRequest::OFFSET_OLDPATH + oldpath_length + newpath_length)]
                .to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid new path"))?;

        Ok(Self {
            olddirfd,
            oldpath,
            newdirfd,
            newpath,
            flags,
        })
    }
}

impl MessagePartitioner for LinkAtRequest {
    ///
    /// # Description
    ///
    /// Partitions a request message for the `linkat()` system call.
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
    /// Upon success, the partitioned message is returned. Upon failure, an error is returned.
    ///
    fn new_part(
        pid: ProcessIdentifier,
        part_number: u32,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_request(
            pid,
            LinuxDaemonMessageHeader::LinkAtRequestPart,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// LinkAtResponse
//==================================================================================================

#[repr(C, packed)]
pub struct LinkAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(LinkAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl LinkAtResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    pub fn new(ret: i32) -> Self {
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
        let message: LinkAtResponse = LinkAtResponse::new(ret);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::LinkAtResponse, message.into_bytes());
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());
        message
    }
}
