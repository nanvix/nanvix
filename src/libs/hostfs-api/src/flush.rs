// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Flush request wire format.

use crate::{
    set_kind,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
};
use ::sys::ipc::Message;

/// Flush request: flush pending writes.
#[derive(Debug, Clone, Copy)]
pub struct FlushRequest {
    /// Remote file descriptor.
    pub fd: i32,
}

impl FlushRequest {
    /// Serializes this request into a complete message payload (header + op_id + data).
    pub fn serialize(&self, kind_value: u16, op_id: OperationId) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_kind(&mut payload, kind_value);
        set_op_id(&mut payload, op_id);
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload
    }

    /// Decodes a FlushRequest from the message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        Self { fd }
    }
}
