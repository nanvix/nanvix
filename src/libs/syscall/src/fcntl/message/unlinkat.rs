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
use ::core::mem;
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
    limits::NAME_MAX,
};

//==================================================================================================
// UnlinkAtRequest
//==================================================================================================

///
/// # Description
///
/// This structure represents the request message of the `unlinkat()` system call.
///
#[derive(Debug)]
pub struct UnlinkAtRequest {
    pub dirfd: i32,
    pub flags: c_int,
    pub pathname: String,
}

impl UnlinkAtRequest {
    /// Size of `dirfd` field.
    pub const SIZE_OF_DIRFD: usize = mem::size_of::<i32>();
    /// Size of `flags` field.
    pub const SIZE_OF_FLAGS: usize = mem::size_of::<c_int>();
    /// Size of `pathname.len()` field.
    pub const SIZE_OF_PATHNAME_LEN: usize = mem::size_of::<u32>();
    /// Offset of `dirfd` field.
    pub const OFFSET_OF_DIRFD: usize = 0;
    /// Offset of `flags` field.
    pub const OFFSET_OF_FLAGS: usize = Self::OFFSET_OF_DIRFD + Self::SIZE_OF_DIRFD;
    /// Offset of `pathname.len()` field.
    pub const OFFSET_OF_PATHNAME_LEN: usize = Self::OFFSET_OF_FLAGS + Self::SIZE_OF_FLAGS;
    /// Offset of `pathname` field.
    pub const OFFSET_OF_PATHNAME: usize = Self::OFFSET_OF_PATHNAME_LEN + Self::SIZE_OF_PATHNAME_LEN;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize =
        Self::SIZE_OF_DIRFD + Self::SIZE_OF_FLAGS + Self::SIZE_OF_PATHNAME_LEN + NAME_MAX;

    ///
    /// # Description
    ///
    /// Creates a new request message for the `unlinkat()` system call.
    ///
    /// # Parameters
    ///
    /// * `dirfd` - Directory file descriptor.
    /// * `pathname` - Pathname of the file to be unlinked.
    /// * `flags` - Flags for the unlink operation.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the function returns the request message. Otherwise, it returns
    /// an error.
    ///
    pub fn new(dirfd: i32, pathname: &str, flags: c_int) -> Result<Self, Error> {
        // Check if pathname is too long.
        if pathname.len() > NAME_MAX {
            #[cfg(target_os = "none")]
            ::syslog::error!(
                "new(): pathname is too long (dirfd={:?}, pathname={:?}, flags={:?})",
                dirfd,
                pathname,
                flags
            );
            return Err(Error::new(ErrorCode::InvalidArgument, "pathname is too long"));
        }

        Ok(Self {
            dirfd,
            flags,
            pathname: pathname.to_string(),
        })
    }
}

impl MessageSerializer for UnlinkAtRequest {
    /// Serializes a request message for the `unlinkat()` system call.
    fn to_bytes(&self) -> Vec<u8> {
        // Allocate buffer.
        let mut buffer: Vec<u8> = Vec::new();

        // Serialize `dirfd` field.
        buffer.extend_from_slice(&self.dirfd.to_le_bytes());
        // Serialize `flags` field.
        buffer.extend_from_slice(&self.flags.to_le_bytes());
        let pathname_bytes: &[u8] = self.pathname.as_bytes();
        // Serialize `pathname.len()` field.
        buffer.extend_from_slice(&(pathname_bytes.len() as u32).to_le_bytes());
        // Serialize `pathname` field.
        buffer.extend_from_slice(pathname_bytes);

        buffer
    }
}

impl MessageDeserializer for UnlinkAtRequest {
    /// Deserializes a request messages for the `unlinkat()` system call.
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the message is too short.
        if bytes.len() < Self::OFFSET_OF_PATHNAME {
            #[cfg(target_os = "none")]
            ::syslog::error!("try_from_bytes(): message is too short (len={:?})", bytes.len());
            return Err(Error::new(ErrorCode::InvalidArgument, "message is too short"));
        }

        // Check if the message is too long.
        if bytes.len() > Self::MAX_SIZE {
            #[cfg(target_os = "none")]
            ::syslog::error!("try_from_bytes(): message is too long (len={:?})", bytes.len());
            return Err(Error::new(ErrorCode::InvalidArgument, "message is too long"));
        }

        // Deserialize `dirfd` field.
        let dirfd: i32 = i32::from_le_bytes(
            bytes[Self::OFFSET_OF_DIRFD..Self::OFFSET_OF_DIRFD + Self::SIZE_OF_DIRFD]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidArgument, "failed to deserialize dirfd")
                })?,
        );
        // Deserialize `flags` field.
        let flags: i32 = c_int::from_le_bytes(
            bytes[Self::OFFSET_OF_FLAGS..Self::OFFSET_OF_FLAGS + Self::SIZE_OF_FLAGS]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidArgument, "failed to deserialize flags")
                })?,
        );
        // Deserialize `pathname.len()` field.
        let pathname_len: usize = u32::from_le_bytes(
            bytes[Self::OFFSET_OF_PATHNAME_LEN
                ..Self::OFFSET_OF_PATHNAME_LEN + Self::SIZE_OF_PATHNAME_LEN]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidArgument, "failed to deserialize pathname length")
                })?,
        ) as usize;

        // Check if the message is too short.
        if bytes.len() < Self::OFFSET_OF_PATHNAME + pathname_len {
            #[cfg(target_os = "none")]
            ::syslog::error!("try_from_bytes(): message is too short (len={:?})", bytes.len());
            return Err(Error::new(ErrorCode::InvalidArgument, "message is too short"));
        }

        // Check if `pathname` is too long.
        if pathname_len > NAME_MAX {
            #[cfg(target_os = "none")]
            ::syslog::error!("try_from_bytes(): pathname is too long (len={:?})", pathname_len);
            return Err(Error::new(ErrorCode::InvalidArgument, "pathname is too long"));
        }

        // Deserialize `pathname` field.
        let pathname: String = String::from_utf8(
            bytes[Self::OFFSET_OF_PATHNAME..Self::OFFSET_OF_PATHNAME + pathname_len].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to deserialize pathname"))?;

        Ok(Self {
            dirfd,
            flags,
            pathname,
        })
    }
}

impl MessagePartitioner for UnlinkAtRequest {
    /// Creates a new message part for the `unlinkat()` system call.
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_request(
            tid,
            LinuxDaemonMessageHeader::UnlinkAtRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// UnlinkAtResponse
//==================================================================================================

#[repr(C, packed)]
pub struct UnlinkAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(UnlinkAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl UnlinkAtResponse {
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

    pub fn build(tid: ThreadIdentifier, ret: i32) -> Message {
        let message: UnlinkAtResponse = UnlinkAtResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::UnlinkAtResponse,
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
