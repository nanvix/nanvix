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
use ::sysapi::limits::NAME_MAX;

//==================================================================================================
// RenameAtRequest
//==================================================================================================

///
/// # Description
///
/// This structure represents the request message of the `renameat()` system call.
///
#[derive(Debug)]
pub struct RenameAtRequest {
    pub olddirfd: i32,
    pub newdirfd: i32,
    pub oldpath: String,
    pub newpath: String,
}

impl RenameAtRequest {
    /// Size of `olddirfd` field.
    pub const SIZE_OF_OLDDIRFD: usize = mem::size_of::<i32>();
    /// Size of `newdirfd` field.
    pub const SIZE_OF_NEWDIRFD: usize = mem::size_of::<i32>();
    /// Size of `oldpath.len()` field.
    pub const SIZE_OF_OLDPATH_LEN: usize = mem::size_of::<u32>();
    /// Size of `newpath.len()` field.
    pub const SIZEO_OF_NEWPATH_LEN: usize = mem::size_of::<u32>();
    /// Offset of `olddirfd` field.
    pub const OFFSET_OLDDIRFD: usize = 0;
    /// Offset of `newdirfd` field.
    pub const OFFSET_NEWDIRFD: usize = Self::OFFSET_OLDDIRFD + Self::SIZE_OF_OLDDIRFD;
    /// Offset of `oldpath.len()` field.
    pub const OFFSET_OLDPATH_LEN: usize = Self::OFFSET_NEWDIRFD + Self::SIZE_OF_NEWDIRFD;
    /// Offset of `newpath.len()` field.
    pub const OFFSET_NEWPATH_LEN: usize = Self::OFFSET_OLDPATH_LEN + Self::SIZE_OF_OLDPATH_LEN;
    /// Offset of `oldpath` field.
    pub const OFFSET_OLDPATH: usize = Self::OFFSET_NEWPATH_LEN + Self::SIZEO_OF_NEWPATH_LEN;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::SIZE_OF_OLDDIRFD
        + Self::SIZE_OF_NEWDIRFD
        + Self::SIZE_OF_OLDPATH_LEN
        + Self::SIZEO_OF_NEWPATH_LEN
        + NAME_MAX
        + NAME_MAX;

    ///
    /// # Description
    ///
    /// Creates a new request message for the `renameat()` system call.
    ///
    /// # Parameters
    ///
    /// - `olddirfd`: Directory file descriptor of the old file.
    /// - `oldpath`:  Pathname of the old file.
    /// - `newdirfd`: Directory file descriptor of the new file.
    /// - `newpath`:  Pathname of the new file.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the `renameat()` system call returns empty. Otherwise, it
    /// returns an error.
    ///
    pub fn new(olddirfd: i32, oldpath: &str, newdirfd: i32, newpath: &str) -> Result<Self, Error> {
        // Check if `oldpath` is too long.
        if oldpath.len() > NAME_MAX {
            #[cfg(target_os = "none")]
            ::syslog::error!(
                "renameat(): oldpath is too long (olddirfd={:?}, oldpath={:?}, newdirfd={:?}, \
                 newpath={:?})",
                olddirfd,
                oldpath,
                newdirfd,
                newpath
            );
            return Err(Error::new(ErrorCode::NameTooLong, "renameat(): oldpath is too long"));
        }

        // Check if `newpath` is too long.
        if newpath.len() > NAME_MAX {
            #[cfg(target_os = "none")]
            ::syslog::error!(
                "renameat(): newpath is too long (olddirfd={:?}, oldpath={:?}, newdirfd={:?}, \
                 newpath={:?})",
                olddirfd,
                oldpath,
                newdirfd,
                newpath
            );
            return Err(Error::new(ErrorCode::NameTooLong, "renameat(): newpath is too long"));
        }

        Ok(Self {
            olddirfd,
            newdirfd,
            oldpath: oldpath.to_string(),
            newpath: newpath.to_string(),
        })
    }
}

impl MessageSerializer for RenameAtRequest {
    // Serializes a request message for the `renameat()` system call.
    fn to_bytes(&self) -> Vec<u8> {
        // Allocate buffer.
        let mut buffer: Vec<u8> = Vec::new();

        // Serialize `olddirfd` field.
        buffer.extend_from_slice(&self.olddirfd.to_le_bytes());
        // Serialize `newdirfd` field.
        buffer.extend_from_slice(&self.newdirfd.to_le_bytes());
        let old_path_bytes: &[u8] = self.oldpath.as_bytes();
        let new_path_bytes: &[u8] = self.newpath.as_bytes();
        // Serialize `oldpath.len()` field.
        buffer.extend_from_slice(&(old_path_bytes.len() as u32).to_le_bytes());
        // Serialize `newpath.len()` field.
        buffer.extend_from_slice(&(new_path_bytes.len() as u32).to_le_bytes());
        // Serialize `oldpath` field.
        buffer.extend_from_slice(old_path_bytes);
        // Serialize `newpath` field.
        buffer.extend_from_slice(new_path_bytes);

        buffer
    }
}

impl MessageDeserializer for RenameAtRequest {
    // Deserializes a request message for the `renameat()` system call.
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the message is too short.
        if bytes.len() < Self::OFFSET_OLDPATH {
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

        // Deserialize `olddirfd` field.
        let olddirfd: i32 = i32::from_le_bytes(
            bytes[Self::OFFSET_OLDDIRFD..(Self::OFFSET_OLDDIRFD + Self::SIZE_OF_OLDDIRFD)]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidArgument, "failed to deserialize olddirfd")
                })?,
        );
        // Deserialize `newdirfd` field.
        let newdirfd: i32 = i32::from_le_bytes(
            bytes[Self::OFFSET_NEWDIRFD..(Self::OFFSET_NEWDIRFD + Self::SIZE_OF_NEWDIRFD)]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidArgument, "failed to deserialize newdirfd")
                })?,
        );
        // Deserialize `oldpath.len()` field.
        let oldpath_len: u32 = u32::from_le_bytes(
            bytes[Self::OFFSET_OLDPATH_LEN..Self::OFFSET_OLDPATH_LEN + Self::SIZE_OF_OLDPATH_LEN]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidArgument, "failed to deserialize oldpath.len()")
                })?,
        );
        // Deserialize `newpath.len()` field.
        let newpath_len: u32 = u32::from_le_bytes(
            bytes[Self::OFFSET_NEWPATH_LEN..Self::OFFSET_NEWPATH_LEN + Self::SIZEO_OF_NEWPATH_LEN]
                .try_into()
                .map_err(|_| {
                    Error::new(ErrorCode::InvalidArgument, "failed to deserialize newpath.len()")
                })?,
        );

        // Check if the message is too short.
        if bytes.len() < Self::OFFSET_OLDPATH + oldpath_len as usize {
            #[cfg(target_os = "none")]
            ::syslog::error!("try_from_bytes(): message is too short (len={:?})", bytes.len());
            return Err(Error::new(ErrorCode::InvalidArgument, "message is too short"));
        }

        // Check if `oldpath` is too long.
        if oldpath_len as usize > NAME_MAX {
            #[cfg(target_os = "none")]
            ::syslog::error!(
                "try_from_bytes(): oldpath is too long (olddirfd={:?}, oldpath={:?}, \
                 newdirfd={:?}, newpath={:?})",
                olddirfd,
                oldpath_len,
                newdirfd,
                newpath_len
            );
            return Err(Error::new(ErrorCode::NameTooLong, "renameat(): oldpath is too long"));
        }

        // Deserialize `oldpath` field.
        let oldpath: String = String::from_utf8(
            bytes[Self::OFFSET_OLDPATH..(Self::OFFSET_OLDPATH + oldpath_len as usize)].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to deserialize oldpath"))?;

        // Check if the message is too short.
        if bytes.len() < Self::OFFSET_OLDPATH + oldpath_len as usize + newpath_len as usize {
            #[cfg(target_os = "none")]
            ::syslog::error!("try_from_bytes(): message is too short (len={:?})", bytes.len());
            return Err(Error::new(ErrorCode::InvalidArgument, "message is too short"));
        }

        // Check if `newpath` is too long.
        if newpath_len as usize > NAME_MAX {
            #[cfg(target_os = "none")]
            ::syslog::error!(
                "try_from_bytes(): newpath is too long (olddirfd={:?}, oldpath={:?}, \
                 newdirfd={:?}, newpath={:?})",
                olddirfd,
                oldpath,
                newdirfd,
                newpath_len
            );
            return Err(Error::new(ErrorCode::NameTooLong, "renameat(): newpath is too long"));
        }

        // Deserialize `newpath` field.
        let newpath: String = String::from_utf8(
            bytes[(Self::OFFSET_OLDPATH + oldpath_len as usize)
                ..(Self::OFFSET_OLDPATH + oldpath_len as usize + newpath_len as usize)]
                .to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to deserialize newpath"))?;

        Ok(Self {
            olddirfd,
            newdirfd,
            oldpath,
            newpath,
        })
    }
}

impl MessagePartitioner for RenameAtRequest {
    /// Creates a new message for the `renameat()` system call.
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_request(
            tid,
            LinuxDaemonMessageHeader::RenameAtRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// RenameAtResponse
//==================================================================================================

#[repr(C, packed)]
pub struct RenameAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(RenameAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl RenameAtResponse {
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
        let message: RenameAtResponse = RenameAtResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::RenameAtResponse,
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
