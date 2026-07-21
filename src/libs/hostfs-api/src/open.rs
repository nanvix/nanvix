// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Open request and response wire format.

use crate::{
    set_header,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
    MAX_INLINE_PATH_LEN,
};
use ::sys::ipc::Message;

/// Maximum path length in an inline open request.
const MAX_INLINE_OPEN_PATH_LEN: usize = MAX_INLINE_PATH_LEN - 4;

/// Open request: open a file at the given relative path.
#[derive(Debug, Clone)]
pub struct OpenRequest {
    /// POSIX open flags (O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, etc.).
    pub flags: i32,
    /// Mode used when creating a file.
    pub mode: u32,
    /// Relative path within the mounted directory.
    pub path_len: u16,
    /// Path bytes.
    pub path: [u8; MAX_INLINE_OPEN_PATH_LEN],
}

/// Open response: contains the remote file descriptor and directory flag.
#[derive(Debug, Clone, Copy)]
pub struct OpenResponse {
    /// Remote file descriptor (negative on error).
    pub fd: i32,
    /// Whether the opened path is a directory (1 = directory, 0 = file).
    pub is_dir: u8,
}

impl OpenRequest {
    /// Builds an inline [`OpenRequest`] from a path slice, POSIX `flags`, and creation `mode`.
    ///
    /// Returns `None` if `path` is longer than the inline request capacity. Callers must
    /// fall back to the multi-part request form in [`long_msg`](crate::long_msg) when
    /// this returns `None`.
    pub fn from_path(flags: i32, mode: u32, path: &[u8]) -> Option<Self> {
        if path.len() > MAX_INLINE_OPEN_PATH_LEN {
            return None;
        }
        let mut buf: [u8; MAX_INLINE_OPEN_PATH_LEN] = [0u8; MAX_INLINE_OPEN_PATH_LEN];
        buf[..path.len()].copy_from_slice(path);
        Some(Self {
            flags,
            mode,
            path_len: path.len() as u16,
            path: buf,
        })
    }

    /// Serializes this request into a complete message payload (header + op_id + data).
    pub fn serialize(&self, header_value: u16, op_id: OperationId) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_header(&mut payload, header_value);
        set_op_id(&mut payload, op_id);
        let data_start: usize = HOSTFS_DATA_START;
        let flags_bytes: [u8; 4] = self.flags.to_le_bytes();
        let mode_bytes: [u8; 4] = self.mode.to_le_bytes();
        let path_len_bytes: [u8; 2] = self.path_len.to_le_bytes();
        payload[data_start..data_start + 4].copy_from_slice(&flags_bytes);
        payload[data_start + 4..data_start + 8].copy_from_slice(&mode_bytes);
        payload[data_start + 8..data_start + 10].copy_from_slice(&path_len_bytes);
        let path_copy_len: usize = (self.path_len as usize).min(MAX_INLINE_OPEN_PATH_LEN);
        payload[data_start + 10..data_start + 10 + path_copy_len]
            .copy_from_slice(&self.path[..path_copy_len]);
        payload
    }

    /// Decodes an OpenRequest from the operation data portion of a message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let flags: i32 =
            i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let mode: u32 =
            u32::from_le_bytes(payload[data_start + 4..data_start + 8].try_into().unwrap());
        let path_len: u16 =
            u16::from_le_bytes(payload[data_start + 8..data_start + 10].try_into().unwrap());
        let mut path: [u8; MAX_INLINE_OPEN_PATH_LEN] = [0u8; MAX_INLINE_OPEN_PATH_LEN];
        let copy_len: usize = (path_len as usize).min(MAX_INLINE_OPEN_PATH_LEN);
        path[..copy_len].copy_from_slice(&payload[data_start + 10..data_start + 10 + copy_len]);
        Self {
            flags,
            mode,
            path_len,
            path,
        }
    }
}

impl OpenResponse {
    /// Encodes this response into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload[data_start + 4] = self.is_dir;
    }

    /// Decodes an OpenResponse from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let is_dir: u8 = payload[data_start + 4];
        Self { fd, is_dir }
    }
}
