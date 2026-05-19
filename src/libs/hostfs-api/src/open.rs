// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Open request and response wire format.

use crate::{
    HOSTFS_DATA_START,
    MAX_INLINE_PATH_LEN,
};
use ::sys::ipc::Message;

/// Open request: open a file at the given relative path.
#[derive(Debug, Clone)]
pub struct OpenRequest {
    /// POSIX open flags (O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, etc.).
    pub flags: i32,
    /// Relative path within the mounted directory.
    pub path_len: u16,
    /// Path bytes (up to `MAX_INLINE_PATH_LEN`).
    pub path: [u8; MAX_INLINE_PATH_LEN],
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
    /// Encodes this request into the operation data portion of a message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        let flags_bytes: [u8; 4] = self.flags.to_le_bytes();
        let path_len_bytes: [u8; 2] = self.path_len.to_le_bytes();
        payload[data_start..data_start + 4].copy_from_slice(&flags_bytes);
        payload[data_start + 4..data_start + 6].copy_from_slice(&path_len_bytes);
        let path_copy_len: usize = (self.path_len as usize).min(MAX_INLINE_PATH_LEN);
        payload[data_start + 6..data_start + 6 + path_copy_len]
            .copy_from_slice(&self.path[..path_copy_len]);
    }

    /// Decodes an OpenRequest from the operation data portion of a message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let flags: i32 =
            i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let path_len: u16 =
            u16::from_le_bytes(payload[data_start + 4..data_start + 6].try_into().unwrap());
        let mut path: [u8; MAX_INLINE_PATH_LEN] = [0u8; MAX_INLINE_PATH_LEN];
        let copy_len: usize = (path_len as usize).min(MAX_INLINE_PATH_LEN);
        path[..copy_len].copy_from_slice(&payload[data_start + 6..data_start + 6 + copy_len]);
        Self {
            flags,
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
