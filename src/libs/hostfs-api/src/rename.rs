// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Rename request wire format.

use crate::{
    HOSTFS_DATA_START,
    MAX_INLINE_PATH_LEN,
};
use ::sys::ipc::Message;

/// Rename request: rename a file or directory.
#[derive(Debug, Clone)]
pub struct RenameRequest {
    /// Length of the old path.
    pub old_path_len: u16,
    /// Length of the new path.
    pub new_path_len: u16,
    /// Concatenated old + new path bytes (up to 40 bytes total).
    pub paths: [u8; MAX_INLINE_PATH_LEN],
}

impl RenameRequest {
    /// Encodes this request into the message payload.
    ///
    /// If the combined path lengths exceed [`MAX_INLINE_PATH_LEN`], the recorded
    /// `old_path_len` and `new_path_len` are saturated to match the number of bytes
    /// actually written, preventing inconsistency between header fields and data.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        // Widen to usize before adding to avoid u16 overflow.
        let total_len: usize =
            (self.old_path_len as usize + self.new_path_len as usize).min(MAX_INLINE_PATH_LEN);
        // Saturate the recorded lengths to match the truncated buffer.
        let old_len: usize = (self.old_path_len as usize).min(total_len);
        let new_len: usize = total_len.saturating_sub(old_len);
        payload[data_start..data_start + 2].copy_from_slice(&(old_len as u16).to_le_bytes());
        payload[data_start + 2..data_start + 4].copy_from_slice(&(new_len as u16).to_le_bytes());
        payload[data_start + 4..data_start + 4 + total_len]
            .copy_from_slice(&self.paths[..total_len]);
    }

    /// Decodes a RenameRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let old_path_len: u16 =
            u16::from_le_bytes(payload[data_start..data_start + 2].try_into().unwrap());
        let new_path_len: u16 =
            u16::from_le_bytes(payload[data_start + 2..data_start + 4].try_into().unwrap());
        let mut paths: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
        // Widen to usize before adding to avoid u16 overflow.
        let total_len: usize =
            (old_path_len as usize + new_path_len as usize).min(MAX_INLINE_PATH_LEN);
        paths[..total_len].copy_from_slice(&payload[data_start + 4..data_start + 4 + total_len]);
        Self {
            old_path_len,
            new_path_len,
            paths,
        }
    }
}
