// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Close request wire format.

use crate::HOSTFS_DATA_START;
use ::sys::ipc::Message;

/// Close request: close a remote file descriptor.
#[derive(Debug, Clone, Copy)]
pub struct CloseRequest {
    /// Remote file descriptor to close.
    pub fd: i32,
}

impl CloseRequest {
    /// Encodes this request into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
    }

    /// Decodes a CloseRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        Self { fd }
    }
}
