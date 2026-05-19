// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Write request and response wire format.

use crate::{
    HOSTFS_DATA_START,
    MAX_INLINE_WRITE_DATA,
};
use ::sys::ipc::Message;

/// Write request: write bytes to a file.
#[derive(Debug, Clone, Copy)]
pub struct WriteRequest {
    /// Remote file descriptor.
    pub fd: i32,
    /// Number of bytes to write.
    pub count: u32,
    /// File offset for positional write (-1 for current position).
    pub offset: i64,
    /// Inline data (up to `MAX_INLINE_WRITE_DATA` bytes).
    pub data_len: u16,
    /// Inline write data.
    pub data: [u8; MAX_INLINE_WRITE_DATA],
}

/// Write response: contains number of bytes written.
#[derive(Debug, Clone, Copy)]
pub struct WriteResponse {
    /// Number of bytes written (negative on error).
    pub bytes_written: i32,
}

impl WriteRequest {
    /// Encodes this request into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload[data_start + 4..data_start + 8].copy_from_slice(&self.count.to_le_bytes());
        payload[data_start + 8..data_start + 16].copy_from_slice(&self.offset.to_le_bytes());
        let len_bytes: [u8; 2] = self.data_len.to_le_bytes();
        payload[data_start + 16..data_start + 18].copy_from_slice(&len_bytes);
        let copy_len: usize = (self.data_len as usize).min(MAX_INLINE_WRITE_DATA);
        payload[data_start + 18..data_start + 18 + copy_len]
            .copy_from_slice(&self.data[..copy_len]);
    }

    /// Decodes a WriteRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let count: u32 =
            u32::from_le_bytes(payload[data_start + 4..data_start + 8].try_into().unwrap());
        let offset: i64 =
            i64::from_le_bytes(payload[data_start + 8..data_start + 16].try_into().unwrap());
        let data_len: u16 = u16::from_le_bytes(
            payload[data_start + 16..data_start + 18]
                .try_into()
                .unwrap(),
        );
        let mut data: [u8; MAX_INLINE_WRITE_DATA] = [0u8; MAX_INLINE_WRITE_DATA];
        let copy_len: usize = (data_len as usize).min(MAX_INLINE_WRITE_DATA);
        data[..copy_len].copy_from_slice(&payload[data_start + 18..data_start + 18 + copy_len]);
        Self {
            fd,
            count,
            offset,
            data_len,
            data,
        }
    }
}

impl WriteResponse {
    /// Encodes this response into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.bytes_written.to_le_bytes());
    }

    /// Decodes a WriteResponse from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let bytes_written: i32 =
            i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        Self { bytes_written }
    }
}
