// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Unlink request wire format.

use crate::{
    HOSTFS_DATA_START,
    MAX_INLINE_PATH_LEN,
};
use ::sys::ipc::Message;

/// Unlink request: remove a file.
#[derive(Debug, Clone)]
pub struct UnlinkRequest {
    /// Relative path of the file to remove.
    pub path_len: u16,
    /// Path bytes.
    pub path: [u8; MAX_INLINE_PATH_LEN],
}

impl UnlinkRequest {
    /// Encodes this request into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 2].copy_from_slice(&self.path_len.to_le_bytes());
        let copy_len: usize = (self.path_len as usize).min(MAX_INLINE_PATH_LEN);
        payload[data_start + 2..data_start + 2 + copy_len].copy_from_slice(&self.path[..copy_len]);
    }

    /// Decodes an UnlinkRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let path_len: u16 =
            u16::from_le_bytes(payload[data_start..data_start + 2].try_into().unwrap());
        let mut path: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
        let copy_len: usize = (path_len as usize).min(MAX_INLINE_PATH_LEN);
        path[..copy_len].copy_from_slice(&payload[data_start + 2..data_start + 2 + copy_len]);
        Self { path_len, path }
    }
}
