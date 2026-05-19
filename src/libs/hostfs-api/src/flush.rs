// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Flush request wire format.

use crate::HOSTFS_DATA_START;
use ::sys::ipc::Message;

/// Flush request: flush pending writes.
#[derive(Debug, Clone, Copy)]
pub struct FlushRequest {
    /// Remote file descriptor.
    pub fd: i32,
}

impl FlushRequest {
    /// Encodes this request into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
    }

    /// Decodes a FlushRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        Self { fd }
    }
}
