// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! ReadDir request and response wire format.

use crate::{
    set_kind,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
    MAX_DIR_ENTRY_NAME_LEN,
};
use ::sys::ipc::Message;

/// ReadDir request: list directory entries.
///
/// The current implementation returns one entry per request using offset-based
/// iteration.
#[derive(Debug, Clone, Copy)]
pub struct ReadDirRequest {
    /// Remote directory file descriptor.
    pub fd: i32,
    /// Reserved for future batched readdir (currently ignored by the handler).
    pub _reserved: u32,
    /// Offset (number of entries to skip) for iterating the directory.
    pub offset: u32,
}

/// ReadDir response: contains a single directory entry.
#[derive(Debug, Clone)]
pub struct ReadDirEntry {
    /// Entry name length.
    pub name_len: u16,
    /// Whether this entry is a directory.
    pub is_dir: u8,
    /// File size in bytes.
    pub size: u64,
    /// Entry name bytes (up to `MAX_DIR_ENTRY_NAME_LEN` bytes inline).
    pub name: [u8; MAX_DIR_ENTRY_NAME_LEN],
}

impl ReadDirRequest {
    /// Serializes this request into a complete message payload (header + op_id + data).
    pub fn serialize(&self, kind_value: u16, op_id: OperationId) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_kind(&mut payload, kind_value);
        set_op_id(&mut payload, op_id);
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload[data_start + 4..data_start + 8].copy_from_slice(&self._reserved.to_le_bytes());
        payload[data_start + 8..data_start + 12].copy_from_slice(&self.offset.to_le_bytes());
        payload
    }

    /// Decodes a ReadDirRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let _reserved: u32 =
            u32::from_le_bytes(payload[data_start + 4..data_start + 8].try_into().unwrap());
        let offset: u32 =
            u32::from_le_bytes(payload[data_start + 8..data_start + 12].try_into().unwrap());
        Self {
            fd,
            _reserved,
            offset,
        }
    }
}

impl ReadDirEntry {
    /// Encodes this entry into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 2].copy_from_slice(&self.name_len.to_le_bytes());
        payload[data_start + 2] = self.is_dir;
        payload[data_start + 3..data_start + 11].copy_from_slice(&self.size.to_le_bytes());
        let copy_len: usize = (self.name_len as usize).min(MAX_DIR_ENTRY_NAME_LEN);
        payload[data_start + 11..data_start + 11 + copy_len]
            .copy_from_slice(&self.name[..copy_len]);
    }

    /// Decodes a ReadDirEntry from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let name_len: u16 =
            u16::from_le_bytes(payload[data_start..data_start + 2].try_into().unwrap());
        let is_dir: u8 = payload[data_start + 2];
        let size: u64 =
            u64::from_le_bytes(payload[data_start + 3..data_start + 11].try_into().unwrap());
        let mut name: [u8; MAX_DIR_ENTRY_NAME_LEN] = [0u8; MAX_DIR_ENTRY_NAME_LEN];
        let copy_len: usize = (name_len as usize).min(MAX_DIR_ENTRY_NAME_LEN);
        name[..copy_len].copy_from_slice(&payload[data_start + 11..data_start + 11 + copy_len]);
        Self {
            name_len,
            is_dir,
            size,
            name,
        }
    }
}
