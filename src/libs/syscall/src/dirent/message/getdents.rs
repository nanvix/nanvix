// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::posix_dent,
    message::{
        LinuxDaemonMessagePart,
        MessageDeserializer,
        MessagePartitioner,
        MessageSerializer,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::vec::Vec;
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
    ffi::{
        c_int,
        c_uchar,
    },
    limits::NAME_MAX,
    sys_types::{
        c_size_t,
        ino_t,
        reclen_t,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of entries in a request/response.
const MAX_ENTRIES: usize = 1024;

//==================================================================================================
// Get Directory Entries Request
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct GetDirectoryEntriesRequest {
    pub fd: c_int,
    pub count: u32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(GetDirectoryEntriesRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl GetDirectoryEntriesRequest {
    pub const PADDING_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<c_int>() - mem::size_of::<u32>();

    /// Maximum number of entries in the request.
    pub const MAX_ENTRIES: usize = MAX_ENTRIES;

    fn new(fd: c_int, count: usize) -> Result<Self, Error> {
        // Check if the `count` is not valid.
        if count == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid buffer count"));
        } else if count > Self::MAX_ENTRIES {
            return Err(Error::new(ErrorCode::TooBig, "request is too large"));
        }

        Ok(Self {
            fd,
            count: count as c_size_t,
            _padding: [0; Self::PADDING_SIZE],
        })
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, fd: c_int, count: usize) -> Result<Message, Error> {
        let message: GetDirectoryEntriesRequest = GetDirectoryEntriesRequest::new(fd, count)?;
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::GetDirectoryEntriesRequest,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(crate::LINUXD),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        Ok(message)
    }
}

//==================================================================================================
// Get Directory Entries Response
//==================================================================================================

#[derive(Debug)]
pub struct GetDirectoryEntriesResponse {
    pub entries: Vec<posix_dent>,
}

impl GetDirectoryEntriesResponse {
    /// Maximum number of entries in the response.
    pub const MAX_ENTRIES: usize = MAX_ENTRIES;
    /// Maximum size of message.
    pub const MAX_SIZE: usize = Self::MAX_ENTRIES
        * (mem::size_of::<ino_t>() // d_ino
            + mem::size_of::<reclen_t>() // d_reclen
            + mem::size_of::<c_uchar>() // d_type
            + mem::size_of::<u32>() // d_name length
            + (NAME_MAX + 1) * mem::size_of::<c_uchar>()); // d_name

    pub fn new(entries: Vec<posix_dent>) -> Self {
        GetDirectoryEntriesResponse { entries }
    }
}

impl MessageSerializer for GetDirectoryEntriesResponse {
    ///
    /// # Description
    ///
    /// Serializes the response message of the `getdents()` system call.
    ///
    /// # Returns
    ///
    /// The serialized message.
    ///
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        let len: u32 = self.entries.len() as u32;
        bytes.extend_from_slice(&len.to_le_bytes());

        for entry in &self.entries {
            bytes.extend_from_slice(&entry.d_ino.to_le_bytes());
            bytes.extend_from_slice(&entry.d_reclen.to_le_bytes());
            bytes.push(entry.d_type);
            let d_name: &[u8] = entry.d_name.as_slice();
            let d_name_len: u32 = d_name.len() as u32;
            bytes.extend_from_slice(&d_name_len.to_le_bytes());
            bytes.extend_from_slice(d_name);
        }

        // The serialized message that we build must not exceed the maximum size that we claim.
        debug_assert!(bytes.len() <= Self::MAX_SIZE, "serialized message is too large");

        bytes
    }
}

impl MessageDeserializer for GetDirectoryEntriesResponse {
    ///
    /// # Description
    ///
    /// Deserializes the response message of the `getdents()` system call.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Serialized message.
    ///
    /// # Returns
    ///
    /// Upon success, the deserialized message is returned. Upon failure, an error is returned.
    ///
    #[allow(clippy::field_reassign_with_default)]
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut entries: Vec<posix_dent> = Vec::new();

        let mut offset: usize = 0;
        let count: usize = u32::from_le_bytes(
            bytes[offset..offset + mem::size_of::<u32>()]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid length of entries"))?,
        ) as usize;
        offset += mem::size_of::<u32>();

        for _ in 0..count {
            let mut entry: posix_dent = posix_dent::default();

            entry.d_ino = ino_t::from_le_bytes(
                bytes[offset..offset + mem::size_of::<ino_t>()]
                    .try_into()
                    .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid inode"))?,
            );
            offset += mem::size_of::<ino_t>();

            entry.d_reclen = reclen_t::from_le_bytes(
                bytes[offset..offset + mem::size_of::<reclen_t>()]
                    .try_into()
                    .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid record length"))?,
            );
            offset += mem::size_of::<reclen_t>();

            entry.d_type = bytes[offset];
            offset += mem::size_of::<c_uchar>();

            let d_name_len: usize = u32::from_le_bytes(
                bytes[offset..offset + mem::size_of::<u32>()]
                    .try_into()
                    .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid length of name"))?,
            ) as usize;
            offset += mem::size_of::<u32>();

            // Check if the length of the name is valid.
            if d_name_len > entry.d_name.len() {
                return Err(Error::new(ErrorCode::InvalidMessage, "invalid length of name"));
            }

            entry.d_name[..d_name_len].copy_from_slice(&bytes[offset..offset + d_name_len]);
            offset += d_name_len * mem::size_of::<c_uchar>();

            entries.push(entry);
        }

        Ok(Self::new(entries))
    }
}

impl MessagePartitioner for GetDirectoryEntriesResponse {
    ///
    /// # Description
    ///
    /// Partitions the response message of the `getdents()` system call.
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
    /// Upon success, the partitioned message is returned. Upon failure, an error is returned.
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
            LinuxDaemonMessageHeader::GetDirectoryEntriesResponsePart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}
