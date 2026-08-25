// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::{
        MessageDeserializer,
        MessagePartitioner,
        MessageSerializer,
        SystemCallMessagePart,
    },
    SystemCallMessage,
    SystemCallMessageKind,
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
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use sysapi::limits::PATH_MAX;

//==================================================================================================
// MountRequest
//==================================================================================================

/// Request message for the `mount()` system call.
///
/// Layout: `source_len (u32) | target_len (u32) | fstype_len (u32) | flags (u64) | source | target | fstype`
#[derive(Debug)]
pub struct MountRequest {
    /// Source path (device or host directory; may be empty).
    pub source: String,
    /// Target mount point in the guest VFS.
    pub target: String,
    /// Filesystem type (e.g., "hostfs").
    pub fstype: String,
    /// Mount flags.
    pub flags: u64,
}

impl MountRequest {
    const SIZE_OF_SOURCE_LEN: usize = mem::size_of::<u32>();
    const SIZE_OF_TARGET_LEN: usize = mem::size_of::<u32>();
    const SIZE_OF_FSTYPE_LEN: usize = mem::size_of::<u32>();
    const SIZE_OF_FLAGS: usize = mem::size_of::<u64>();

    const OFFSET_OF_SOURCE_LEN: usize = 0;
    const OFFSET_OF_TARGET_LEN: usize = Self::OFFSET_OF_SOURCE_LEN + Self::SIZE_OF_SOURCE_LEN;
    const OFFSET_OF_FSTYPE_LEN: usize = Self::OFFSET_OF_TARGET_LEN + Self::SIZE_OF_TARGET_LEN;
    const OFFSET_OF_FLAGS: usize = Self::OFFSET_OF_FSTYPE_LEN + Self::SIZE_OF_FSTYPE_LEN;
    const OFFSET_OF_STRINGS: usize = Self::OFFSET_OF_FLAGS + Self::SIZE_OF_FLAGS;

    /// Maximum size of the message (header fields + up to PATH_MAX for each string).
    pub const MAX_SIZE: usize = Self::SIZE_OF_SOURCE_LEN
        + Self::SIZE_OF_TARGET_LEN
        + Self::SIZE_OF_FSTYPE_LEN
        + Self::SIZE_OF_FLAGS
        + PATH_MAX
        + PATH_MAX
        + 64; // fstype is short

    pub fn new(source: String, target: String, fstype: String, flags: u64) -> Result<Self, Error> {
        if source.len() > PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "source path too long"));
        }
        if target.len() > PATH_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "target path too long"));
        }
        if fstype.len() > 64 {
            return Err(Error::new(ErrorCode::InvalidMessage, "fstype too long"));
        }
        Ok(Self {
            source,
            target,
            fstype,
            flags,
        })
    }
}

impl MessageSerializer for MountRequest {
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        let source_bytes: &[u8] = self.source.as_bytes();
        let target_bytes: &[u8] = self.target.as_bytes();
        let fstype_bytes: &[u8] = self.fstype.as_bytes();

        buffer.extend_from_slice(&(source_bytes.len() as u32).to_ne_bytes());
        buffer.extend_from_slice(&(target_bytes.len() as u32).to_ne_bytes());
        buffer.extend_from_slice(&(fstype_bytes.len() as u32).to_ne_bytes());
        buffer.extend_from_slice(&self.flags.to_ne_bytes());
        buffer.extend_from_slice(source_bytes);
        buffer.extend_from_slice(target_bytes);
        buffer.extend_from_slice(fstype_bytes);

        buffer
    }
}

impl MessageDeserializer for MountRequest {
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < Self::OFFSET_OF_STRINGS {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short"));
        }
        if bytes.len() > Self::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too long"));
        }

        let source_len: usize = u32::from_ne_bytes(
            bytes[Self::OFFSET_OF_SOURCE_LEN..Self::OFFSET_OF_TARGET_LEN]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid source_len"))?,
        ) as usize;
        let target_len: usize = u32::from_ne_bytes(
            bytes[Self::OFFSET_OF_TARGET_LEN..Self::OFFSET_OF_FSTYPE_LEN]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid target_len"))?,
        ) as usize;
        let fstype_len: usize = u32::from_ne_bytes(
            bytes[Self::OFFSET_OF_FSTYPE_LEN..Self::OFFSET_OF_FLAGS]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid fstype_len"))?,
        ) as usize;
        let flags: u64 = u64::from_ne_bytes(
            bytes[Self::OFFSET_OF_FLAGS..Self::OFFSET_OF_STRINGS]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid flags"))?,
        );

        let total_str_len: usize = source_len + target_len + fstype_len;
        if bytes.len() < Self::OFFSET_OF_STRINGS + total_str_len {
            return Err(Error::new(ErrorCode::InvalidMessage, "message too short for strings"));
        }
        if source_len > PATH_MAX || target_len > PATH_MAX || fstype_len > 64 {
            return Err(Error::new(ErrorCode::InvalidMessage, "string field too long"));
        }

        let str_start: usize = Self::OFFSET_OF_STRINGS;
        let source: String =
            String::from_utf8(bytes[str_start..str_start + source_len].to_vec())
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid source path"))?;

        let target: String = String::from_utf8(
            bytes[str_start + source_len..str_start + source_len + target_len].to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid target path"))?;

        let fstype: String = String::from_utf8(
            bytes[str_start + source_len + target_len
                ..str_start + source_len + target_len + fstype_len]
                .to_vec(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid fstype"))?;

        Ok(Self {
            source,
            target,
            fstype,
            flags,
        })
    }
}

impl MessagePartitioner for MountRequest {
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; SystemCallMessagePart::PAYLOAD_SIZE],
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Message, Error> {
        SystemCallMessagePart::build_request(
            tid,
            SystemCallMessageKind::HostMountRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
            destination,
            message_type,
        )
    }
}

//==================================================================================================
// MountResponse
//==================================================================================================

#[repr(C, packed)]
pub struct MountResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(MountResponse, SystemCallMessage::PAYLOAD_SIZE);

impl MountResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(ret: i32) -> Self {
        Self {
            ret,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        ret: i32,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: MountResponse = MountResponse::new(ret);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::HostMountResponse, message.into_bytes());
        Message::new(
            MessageSender::new(source, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}
