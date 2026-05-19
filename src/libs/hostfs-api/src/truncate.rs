// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Truncate request wire format.

use crate::HOSTFS_DATA_START;
use ::sys::ipc::Message;

/// Truncate request: truncate a file to a given length.
#[derive(Debug, Clone, Copy)]
pub struct TruncateRequest {
    /// Remote file descriptor.
    pub fd: i32,
    /// New file length.
    pub length: i64,
}

impl TruncateRequest {
    /// Encodes this request into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload[data_start + 4..data_start + 12].copy_from_slice(&self.length.to_le_bytes());
    }

    /// Decodes a TruncateRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let length: i64 =
            i64::from_le_bytes(payload[data_start + 4..data_start + 12].try_into().unwrap());
        Self { fd, length }
    }
}
