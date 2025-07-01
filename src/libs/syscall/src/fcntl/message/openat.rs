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
    limits::PATH_MAX,
    sys_types::mode_t,
};

//==================================================================================================
// OpenAtRequest
//==================================================================================================

///
/// # Description
///
/// This structure represents the request message of the `openat()` system call.
///
#[derive(Debug)]
pub struct OpenAtRequest {
    pub dirfd: i32,
    pub flags: c_int,
    pub mode: mode_t,
    pub pathname: String,
}

impl OpenAtRequest {
    /// Size of `dirfd` field.
    pub const SIZEO_OF_DIRFD: usize = mem::size_of::<i32>();
    /// Size of `flags` field.
    pub const SIZEO_OF_FLAGS: usize = mem::size_of::<c_int>();
    /// Size of `mode` field.
    pub const SIZEO_OF_MODE: usize = mem::size_of::<mode_t>();
    /// Size of `pathname.len()` field.
    pub const SIZEO_OF_PATHNAME_LEN: usize = mem::size_of::<u32>();
    /// Offset of `dirfd` field.
    pub const OFFSET_OF_DIRFD: usize = 0;
    /// Offset of `flags` field.
    pub const OFFSET_OF_FLAGS: usize = Self::OFFSET_OF_DIRFD + Self::SIZEO_OF_DIRFD;
    /// Offset of `mode` field.
    pub const OFFSET_OF_MODE: usize = Self::OFFSET_OF_FLAGS + Self::SIZEO_OF_FLAGS;
    /// Offset of `pathname.len()` field.
    pub const OFFSET_OF_PATHNAME_LEN: usize = Self::OFFSET_OF_MODE + Self::SIZEO_OF_MODE;
    /// Offset of `pathname` field.
    pub const OFFSET_OF_PATHNAME: usize =
        Self::OFFSET_OF_PATHNAME_LEN + Self::SIZEO_OF_PATHNAME_LEN;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = PATH_MAX
        + Self::SIZEO_OF_DIRFD
        + Self::SIZEO_OF_FLAGS
        + Self::SIZEO_OF_MODE
        + Self::SIZEO_OF_PATHNAME_LEN
        + PATH_MAX;

    ///
    /// # Description
    ///
    /// Creates a new request message for the `openat()` system call.
    ///
    /// # Parameters
    ///
    /// - `dirfd`: Directory file descriptor.
    /// - `pathname`: Path.
    /// - `flags`: Flags.
    /// - `mode`: Mode.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the function returns the request message. Otherwise, it returns
    /// an error.
    ///
    pub fn new(dirfd: i32, pathname: &str, flags: c_int, mode: mode_t) -> Result<Self, Error> {
        // Check if the path is too long.
        if pathname.len() > PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidArgument, "path is too long"));
        }

        Ok(Self {
            dirfd,
            flags,
            mode,
            pathname: pathname.to_string(),
        })
    }
}

impl MessageSerializer for OpenAtRequest {
    /// Serializes a request message for the `openat()` system call.
    fn to_bytes(&self) -> Vec<u8> {
        // Allocate buffer.
        let mut buffer: Vec<u8> = Vec::new();

        // Serialize `dirfd` field.
        buffer.extend_from_slice(&self.dirfd.to_le_bytes());
        // Serialize `flags` field.
        buffer.extend_from_slice(&self.flags.to_le_bytes());
        // Serialize `mode` field.
        buffer.extend_from_slice(&self.mode.to_le_bytes());
        let pathname_bytes: &[u8] = self.pathname.as_bytes();
        // Serialize `pathname.len()` field.
        buffer.extend_from_slice(&(pathname_bytes.len() as u32).to_le_bytes());
        // Serialize `pathname` field.
        buffer.extend_from_slice(pathname_bytes);

        buffer
    }
}

impl MessageDeserializer for OpenAtRequest {
    /// Deserializes a request message for the `openat()` system call.
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the buffer is too short.
        if bytes.len() < Self::OFFSET_OF_PATHNAME {
            return Err(Error::new(ErrorCode::InvalidMessage, "buffer is too short"));
        }

        // Check if message is too long.
        if bytes.len() > Self::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message is too long"));
        }

        // Deserialize `dirfd` field.
        let dirfd: i32 = i32::from_le_bytes(
            bytes[Self::OFFSET_OF_DIRFD..Self::OFFSET_OF_FLAGS]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid dirfd"))?,
        );
        // Deserialize `flags` field.
        let flags: c_int = c_int::from_le_bytes(
            bytes[Self::OFFSET_OF_FLAGS..Self::OFFSET_OF_MODE]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid flags"))?,
        );
        // Deserialize `mode` field.
        let mode: mode_t = mode_t::from_le_bytes(
            bytes[Self::OFFSET_OF_MODE..Self::OFFSET_OF_PATHNAME_LEN]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid mode"))?,
        );
        // Deserialize `pathname.len()` field.
        let pathname_len: usize = u32::from_le_bytes(
            bytes[Self::OFFSET_OF_PATHNAME_LEN..Self::OFFSET_OF_PATHNAME]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid pathname length"))?,
        ) as usize;

        // Check if message is too short.
        if bytes.len() < Self::OFFSET_OF_PATHNAME + pathname_len {
            return Err(Error::new(ErrorCode::InvalidMessage, "message is too short"));
        }

        // Check if `pathname` is too long.
        if pathname_len > PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "pathname is too long"));
        }

        // Deserialize `pathname` field.
        let pathname: String = String::from_utf8(
            bytes[Self::OFFSET_OF_PATHNAME..Self::OFFSET_OF_PATHNAME + pathname_len].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid pathname"))?;

        Ok(Self {
            dirfd,
            flags,
            mode,
            pathname,
        })
    }
}

impl MessagePartitioner for OpenAtRequest {
    /// Creates a new message part for the `openat()` system call.
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_request(
            tid,
            LinuxDaemonMessageHeader::OpenAtRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// OpenAtResponse
//==================================================================================================

#[repr(C, packed)]
pub struct OpenAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(OpenAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl OpenAtResponse {
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

    pub fn build(tid: ThreadIdentifier, ret: i32) -> Message {
        let message: OpenAtResponse = OpenAtResponse::new(ret);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::OpenAtResponse, message.into_bytes());
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
