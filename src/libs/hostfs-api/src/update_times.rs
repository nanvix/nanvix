// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! File timestamp request wire format.

use crate::{
    set_kind,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
};
use ::sys::ipc::Message;
use ::sysapi::time::timespec;

const _: () = assert!(HOSTFS_DATA_START + 4 + 2 * timespec::WIRE_SIZE <= Message::PAYLOAD_SIZE);

/// Descriptor-based timestamp update request.
#[derive(Debug, Clone, Copy)]
pub struct UpdateTimesRequest {
    /// Remote file descriptor.
    pub fd: i32,
    /// Access and modification times.
    pub times: [timespec; 2],
}

impl UpdateTimesRequest {
    /// Serializes this request into a complete message payload.
    pub fn serialize(&self, kind_value: u16, op_id: OperationId) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        set_kind(&mut payload, kind_value);
        set_op_id(&mut payload, op_id);
        let data_start: usize = HOSTFS_DATA_START;
        payload[data_start..data_start + 4].copy_from_slice(&self.fd.to_le_bytes());
        payload[data_start + 4..data_start + 4 + timespec::WIRE_SIZE]
            .copy_from_slice(&self.times[0].to_bytes());
        payload[data_start + 4 + timespec::WIRE_SIZE..data_start + 4 + 2 * timespec::WIRE_SIZE]
            .copy_from_slice(&self.times[1].to_bytes());
        payload
    }

    /// Decodes a descriptor-based timestamp update request.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Option<Self> {
        let data_start: usize = HOSTFS_DATA_START;
        let fd: i32 = i32::from_le_bytes(payload[data_start..data_start + 4].try_into().ok()?);
        let times: [timespec; 2] = [
            timespec::try_from_bytes(
                &payload[data_start + 4..data_start + 4 + timespec::WIRE_SIZE],
            )
            .ok()?,
            timespec::try_from_bytes(
                &payload[data_start + 4 + timespec::WIRE_SIZE
                    ..data_start + 4 + 2 * timespec::WIRE_SIZE],
            )
            .ok()?,
        ];
        Some(Self { fd, times })
    }
}
