// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Lseek request and response wire format.

use crate::{
    set_kind,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
};
use ::sys::ipc::Message;

/// Lseek request: seek within a file.
#[derive(Debug, Clone, Copy)]
pub struct LseekRequest {
    /// Remote file descriptor.
    pub fd: i32,
    /// Seek offset.
    pub offset: i64,
    /// Seek whence (SEEK_SET, SEEK_CUR, SEEK_END).
    pub whence: i32,
}

/// Lseek response: contains new file position.
#[derive(Debug, Clone, Copy)]
pub struct LseekResponse {
    /// New file position (negative on error).
    pub offset: i64,
}

impl LseekRequest {
    /// Serializes this request into a complete message payload (header + op_id + data).
    pub fn serialize(&self, kind_value: u16, op_id: OperationId) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_kind(&mut payload, kind_value);
        set_op_id(&mut payload, op_id);
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload[data_start + 4..data_start + 12].copy_from_slice(&self.offset.to_le_bytes());
        payload[data_start + 12..data_start + 16].copy_from_slice(&self.whence.to_le_bytes());
        payload
    }

    /// Decodes a LseekRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let offset: i64 =
            i64::from_le_bytes(payload[data_start + 4..data_start + 12].try_into().unwrap());
        let whence: i32 = i32::from_le_bytes(
            payload[data_start + 12..data_start + 16]
                .try_into()
                .unwrap(),
        );
        Self { fd, offset, whence }
    }
}

impl LseekResponse {
    /// Encodes this response into the message payload.
    pub fn encode(&self, payload: &mut [u8; Message::PAYLOAD_SIZE]) {
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 8].copy_from_slice(&self.offset.to_le_bytes());
    }

    /// Decodes a LseekResponse from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let offset: i64 =
            i64::from_le_bytes(payload[data_start..data_start + 8].try_into().unwrap());
        Self { offset }
    }
}
