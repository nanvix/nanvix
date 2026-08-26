// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Descriptor-based ownership request wire format.

use crate::{
    set_kind,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
};
use ::sys::ipc::Message;

/// Changes the ownership of an open hostfs file.
#[derive(Debug, Clone, Copy)]
pub struct ChownRequest {
    /// Remote file descriptor.
    pub fd: i32,
    /// New user identifier, or `u32::MAX` to leave it unchanged.
    pub owner: u32,
    /// New group identifier, or `u32::MAX` to leave it unchanged.
    pub group: u32,
}

impl ChownRequest {
    /// Serializes this request into a complete message payload.
    pub fn serialize(&self, kind_value: u16, op_id: OperationId) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_kind(&mut payload, kind_value);
        set_op_id(&mut payload, op_id);
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload[data_start + 4..data_start + 8].copy_from_slice(&self.owner.to_le_bytes());
        payload[data_start + 8..data_start + 12].copy_from_slice(&self.group.to_le_bytes());
        payload
    }

    /// Decodes a descriptor-based ownership request.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().unwrap());
        let owner: u32 =
            u32::from_le_bytes(payload[data_start + 4..data_start + 8].try_into().unwrap());
        let group: u32 =
            u32::from_le_bytes(payload[data_start + 8..data_start + 12].try_into().unwrap());
        Self { fd, owner, group }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_op_id;

    #[test]
    fn request_round_trip_preserves_sentinels() {
        let request = ChownRequest {
            fd: 7,
            owner: u32::MAX,
            group: 42,
        };
        let op_id = OperationId::new(11);
        let payload = request.serialize(123, op_id);
        let decoded = ChownRequest::decode(&payload);

        assert_eq!(get_op_id(&payload), op_id);
        assert_eq!(decoded.fd, request.fd);
        assert_eq!(decoded.owner, request.owner);
        assert_eq!(decoded.group, request.group);
    }
}
