// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Stat request and response wire format.

use crate::{
    set_kind,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
};
use ::sys::ipc::Message;

/// Stat request: get file metadata.
#[derive(Debug, Clone, Copy)]
pub struct StatRequest {
    /// Remote file descriptor (-1 to stat by path).
    pub fd: i32,
}

/// Stat response: contains file metadata.
///
/// The `status` field indicates success (0) or a `HOSTFS_ERR_*` error code (negative).
/// On error, the remaining fields (`size`, `mode`, `is_dir`) are undefined.
#[derive(Debug, Clone, Copy)]
pub struct StatResponse {
    /// Status code: 0 on success, negative `HOSTFS_ERR_*` on failure.
    pub status: i32,
    /// File size in bytes.
    pub size: u64,
    /// File mode/permissions.
    pub mode: u32,
    /// Whether the entry is a directory.
    pub is_dir: u8,
}

impl StatRequest {
    /// Serializes this request into a complete message payload (header + op_id + data).
    pub fn serialize(&self, kind_value: u16, op_id: OperationId) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_kind(&mut payload, kind_value);
        set_op_id(&mut payload, op_id);
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload
    }

    /// Decodes a StatRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        Self { fd }
    }
}

impl StatResponse {
    /// Encodes this response into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.status.to_le_bytes());
        payload[data_start + 4..data_start + 12].copy_from_slice(&self.size.to_le_bytes());
        payload[data_start + 12..data_start + 16].copy_from_slice(&self.mode.to_le_bytes());
        payload[data_start + 16] = self.is_dir;
    }

    /// Decodes a StatResponse from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let status: i32 =
            i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let size: u64 =
            u64::from_le_bytes(payload[data_start + 4..data_start + 12].try_into().unwrap());
        let mode: u32 = u32::from_le_bytes(
            payload[data_start + 12..data_start + 16]
                .try_into()
                .unwrap(),
        );
        let is_dir: u8 = payload[data_start + 16];
        Self {
            status,
            size,
            mode,
            is_dir,
        }
    }
}
