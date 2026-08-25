// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Read request and response wire format.

use crate::{
    set_kind,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
    MAX_INLINE_READ_DATA,
};
use ::sys::ipc::Message;

/// Read request: read bytes from a file.
#[derive(Debug, Clone, Copy)]
pub struct ReadRequest {
    /// Remote file descriptor.
    pub fd: i32,
    /// Number of bytes to read.
    pub count: u32,
    /// File offset for positional read (-1 for current position).
    pub offset: i64,
}

/// Read response: contains data read from the file.
#[derive(Debug, Clone, Copy)]
pub struct ReadResponse {
    /// Number of bytes read (negative on error).
    pub bytes_read: i32,
    /// Inline read data (up to `MAX_INLINE_READ_DATA` bytes).
    pub data: [u8; MAX_INLINE_READ_DATA],
}

impl ReadRequest {
    /// Serializes this request into a complete message payload (header + op_id + data).
    pub fn serialize(&self, kind_value: u16, op_id: OperationId) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_kind(&mut payload, kind_value);
        set_op_id(&mut payload, op_id);
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload[data_start + 4..data_start + 8].copy_from_slice(&self.count.to_le_bytes());
        payload[data_start + 8..data_start + 16].copy_from_slice(&self.offset.to_le_bytes());
        payload
    }

    /// Decodes a ReadRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let count: u32 =
            u32::from_le_bytes(payload[data_start + 4..data_start + 8].try_into().unwrap());
        let offset: i64 =
            i64::from_le_bytes(payload[data_start + 8..data_start + 16].try_into().unwrap());
        Self { fd, count, offset }
    }
}

impl ReadResponse {
    /// Encodes this response into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.bytes_read.to_le_bytes());
        let copy_len: usize = (self.bytes_read.max(0) as usize).min(MAX_INLINE_READ_DATA);
        payload[data_start + 4..data_start + 4 + copy_len].copy_from_slice(&self.data[..copy_len]);
    }

    /// Decodes a ReadResponse from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let raw_bytes_read: i32 =
            i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        // Clamp to the inline buffer size to prevent out-of-bounds reads from
        // corrupted or malicious wire data.
        let bytes_read: i32 = if raw_bytes_read > MAX_INLINE_READ_DATA as i32 {
            MAX_INLINE_READ_DATA as i32
        } else {
            raw_bytes_read
        };
        let mut data: [u8; MAX_INLINE_READ_DATA] = [0u8; MAX_INLINE_READ_DATA];
        let copy_len: usize = (bytes_read.max(0) as usize).min(MAX_INLINE_READ_DATA);
        data[..copy_len].copy_from_slice(&payload[data_start + 4..data_start + 4 + copy_len]);
        Self { bytes_read, data }
    }
}
