// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Rmdir request wire format.

use crate::{
    set_header,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
    MAX_INLINE_PATH_LEN,
};
use ::sys::ipc::Message;

/// Rmdir request: remove a directory.
#[derive(Debug, Clone)]
pub struct RmdirRequest {
    /// Relative path of the directory to remove.
    pub path_len: u16,
    /// Path bytes.
    pub path: [u8; MAX_INLINE_PATH_LEN],
}

impl RmdirRequest {
    /// Builds an inline [`RmdirRequest`] from a path slice.
    ///
    /// Returns `None` if `path` is longer than [`MAX_INLINE_PATH_LEN`]. Callers must
    /// fall back to the multi-part request form in [`long_msg`](crate::long_msg) when
    /// this returns `None`.
    pub fn from_path(path: &[u8]) -> Option<Self> {
        if path.len() > MAX_INLINE_PATH_LEN {
            return None;
        }
        let mut buf: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
        buf[..path.len()].copy_from_slice(path);
        Some(Self {
            path_len: path.len() as u16,
            path: buf,
        })
    }

    /// Serializes this request into a complete message payload (header + op_id + data).
    pub fn serialize(
        &self,
        header_value: u16,
        op_id: OperationId,
    ) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_header(&mut payload, header_value);
        set_op_id(&mut payload, op_id);
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 2].copy_from_slice(&self.path_len.to_le_bytes());
        let copy_len: usize = (self.path_len as usize).min(MAX_INLINE_PATH_LEN);
        payload[data_start + 2..data_start + 2 + copy_len].copy_from_slice(&self.path[..copy_len]);
        payload
    }

    /// Decodes a RmdirRequest from the message payload.
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
