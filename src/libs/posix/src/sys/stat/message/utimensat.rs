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
    time::timespec,
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
// UpdateFileAccessTimeAtRequest
//==================================================================================================

///
/// # Description
///
/// This struct represents the request message of the `utimensat()` system call.
///
#[derive(Debug)]
pub struct UpdateFileAccessTimeAtRequest {
    /// Directory file descriptor.
    pub dirfd: i32,
    /// Flags.
    pub flag: i32,
    /// Path.
    pub path: String,
    /// Access time.
    pub times: [timespec; 2],
}

impl UpdateFileAccessTimeAtRequest {
    /// Sizes of 'directory file descriptor' field.
    const SIZE_OF_DIRFD: usize = mem::size_of::<i32>();
    /// Sizes of 'flags' field.
    const SIZE_OF_FLAG: usize = mem::size_of::<i32>();
    /// Sizes of 'path length' field.
    const SIZE_OF_PATH_LENGTH: usize = mem::size_of::<u32>();
    /// Sizes of 'access time' field.
    const SIZE_OF_TIMES: usize = 2 * mem::size_of::<timespec>();
    /// Offsets to 'directory file descriptor' field.
    const OFFSET_TO_DIRFD: usize = 0;
    /// Offsets to 'flags' field.
    const OFFSET_TO_FLAG: usize = Self::OFFSET_TO_DIRFD + Self::SIZE_OF_DIRFD;
    /// Offsets to 'path length' field.
    const OFFSET_TO_PATH_LENGTH: usize = Self::OFFSET_TO_FLAG + Self::SIZE_OF_FLAG;
    /// Offsets to 'access time' field.
    const OFFSET_TO_TIMES: usize = Self::OFFSET_TO_PATH_LENGTH + Self::SIZE_OF_PATH_LENGTH;
    /// Offset to 'path' field.
    const OFFSET_TO_PATH: usize = Self::OFFSET_TO_TIMES + Self::SIZE_OF_TIMES;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::SIZE_OF_DIRFD
        + Self::SIZE_OF_FLAG
        + Self::SIZE_OF_PATH_LENGTH
        + limits::PATH_MAX
        + Self::SIZE_OF_TIMES;

    ///
    /// # Description
    ///
    /// Creates a new request message for the `utimensat()` system call.
    ///
    /// # Parameters
    ///
    /// - `dirfd`: Directory file descriptor.
    /// - `path`: Path.
    /// - `flag`: Flags.
    /// - `times`: Access time.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the new request message. Upon failure, it returns an error.
    ///
    pub fn new(dirfd: i32, path: String, flag: i32, times: [timespec; 2]) -> Result<Self, Error> {
        // Check if path is too long.
        if path.len() > limits::PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "path too long"));
        }

        Ok(UpdateFileAccessTimeAtRequest {
            dirfd,
            flag,
            path,
            times,
        })
    }
}

impl MessageSerializer for UpdateFileAccessTimeAtRequest {
    ///
    /// # Description
    ///
    /// Serializes the request message of the `utimensat()` system call.
    ///
    /// # Returns
    ///
    /// The serialized message.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(Self::MAX_SIZE);

        // Serialize directory file descriptor.
        buffer.extend_from_slice(&self.dirfd.to_ne_bytes());

        // Serialize flags.
        buffer.extend_from_slice(&self.flag.to_ne_bytes());

        // Serialize path length.
        buffer.extend_from_slice(&(self.path.len() as u32).to_ne_bytes());

        // Serialize access time.
        for time in self.times.iter() {
            buffer.extend_from_slice(&time.to_bytes());
        }

        // Serialize path.
        buffer.extend_from_slice(self.path.as_bytes());

        buffer
    }
}

impl MessageDeserializer for UpdateFileAccessTimeAtRequest {
    ///
    /// # Description
    ///
    /// Deserializes the request message of the `utimensat()` system call.
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
        if bytes.len() < Self::OFFSET_TO_PATH {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Check if message is too long.
        if bytes.len() > Self::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too long"));
        }

        // Deserialize directory file descriptor.
        let dirfd: i32 = i32::from_ne_bytes(
            bytes[Self::OFFSET_TO_DIRFD..(Self::OFFSET_TO_DIRFD + Self::SIZE_OF_DIRFD)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid dirfd"))?,
        );

        // Deserialize flags.
        let flag: i32 = i32::from_ne_bytes(
            bytes[Self::OFFSET_TO_FLAG..(Self::OFFSET_TO_FLAG + Self::SIZE_OF_FLAG)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid flag"))?,
        );

        // Deserialize path length.
        let path_length: usize = u32::from_ne_bytes(
            bytes[Self::OFFSET_TO_PATH_LENGTH
                ..(Self::OFFSET_TO_PATH_LENGTH + Self::SIZE_OF_PATH_LENGTH)]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path length"))?,
        ) as usize;

        // Deserialize access time.
        let mut times: [timespec; 2] = [timespec::default(); 2];
        for (i, time) in times.iter_mut().enumerate() {
            let offset: usize = Self::OFFSET_TO_TIMES + i * mem::size_of::<timespec>();
            *time = timespec::try_from_bytes(&bytes[offset..(offset + mem::size_of::<timespec>())])
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid time"))?;
        }

        // Check if message is too short.
        if bytes.len() < Self::OFFSET_TO_PATH + path_length {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }

        // Check if path is too long.
        if path_length > limits::PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "path too long"));
        }

        // Deserialize path.
        let path = String::from_utf8(
            bytes[Self::OFFSET_TO_PATH..Self::OFFSET_TO_PATH + path_length].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid path"))?;

        Ok(UpdateFileAccessTimeAtRequest {
            dirfd,
            flag,
            path,
            times,
        })
    }
}

impl MessagePartitioner for UpdateFileAccessTimeAtRequest {
    ///
    /// # Description
    ///
    /// Creates a new message partition for the `utimensat()` system call.
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
            LinuxDaemonMessageHeader::UpdateFileAccessTimeAtRequestPart,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// UpdateFileAccessTimeAtResponse
//==================================================================================================

///
/// # Description
///
/// This struct represents the response message of the `utimensat()` system call.
///
#[repr(C, packed)]
pub struct UpdateFileAccessTimeAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(UpdateFileAccessTimeAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl UpdateFileAccessTimeAtResponse {
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
        let message: UpdateFileAccessTimeAtResponse = UpdateFileAccessTimeAtResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::UpdateFileAccessTimeAtResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
