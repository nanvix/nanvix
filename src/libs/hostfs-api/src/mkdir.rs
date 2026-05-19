// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Mkdir request wire format.

use crate::{
    HOSTFS_DATA_START,
    MAX_INLINE_PATH_LEN,
};
use ::sys::ipc::Message;

/// Mkdir request: create a directory.
#[derive(Debug, Clone)]
pub struct MkdirRequest {
    /// Directory creation mode.
    pub mode: u32,
    /// Relative path within the mounted directory.
    pub path_len: u16,
    /// Path bytes.
    pub path: [u8; MAX_INLINE_PATH_LEN],
}

impl MkdirRequest {
    /// Encodes this request into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.mode.to_le_bytes());
        payload[data_start + 4..data_start + 6].copy_from_slice(&self.path_len.to_le_bytes());
        let copy_len: usize = (self.path_len as usize).min(MAX_INLINE_PATH_LEN);
        payload[data_start + 6..data_start + 6 + copy_len].copy_from_slice(&self.path[..copy_len]);
    }

    /// Decodes a MkdirRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let mode: u32 = u32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let path_len: u16 =
            u16::from_le_bytes(payload[data_start + 4..data_start + 6].try_into().unwrap());
        let mut path: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
        let copy_len: usize = (path_len as usize).min(MAX_INLINE_PATH_LEN);
        path[..copy_len].copy_from_slice(&payload[data_start + 6..data_start + 6 + copy_len]);
        Self {
            mode,
            path_len,
            path,
        }
    }
}
