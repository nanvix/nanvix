// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Wire format for hostfs descriptor-based permission changes.

use crate::{
    set_kind,
    set_op_id,
    OperationId,
    HOSTFS_DATA_START,
};
use ::core::mem;
use ::sys::ipc::Message;

/// Descriptor-based permission-change request.
#[derive(Debug, Clone, Copy)]
pub struct ChmodRequest {
    /// Remote file descriptor.
    fd: i32,
    /// New file mode.
    mode: u32,
}

impl ChmodRequest {
    /// Size of the file descriptor field.
    const SIZE_OF_FD: usize = mem::size_of::<i32>();
    /// Size of the mode field.
    const SIZE_OF_MODE: usize = mem::size_of::<u32>();
    /// Offset of the file descriptor field.
    const OFFSET_OF_FD: usize = 0;
    /// Offset of the mode field.
    const OFFSET_OF_MODE: usize = Self::OFFSET_OF_FD + Self::SIZE_OF_FD;

    /// Creates a descriptor-based permission-change request.
    pub const fn new(fd: i32, mode: u32) -> Self {
        Self { fd, mode }
    }

    /// Returns the remote file descriptor.
    pub const fn fd(&self) -> i32 {
        self.fd
    }

    /// Returns the requested file mode.
    pub const fn mode(&self) -> u32 {
        self.mode
    }

    /// Serializes this request into a hostfs message payload.
    pub fn serialize(&self, kind_value: u16, op_id: OperationId) -> [u8; Message::PAYLOAD_SIZE] {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0; Message::PAYLOAD_SIZE];
        set_kind(&mut payload, kind_value);
        set_op_id(&mut payload, op_id);
        let fd_offset: usize = HOSTFS_DATA_START + Self::OFFSET_OF_FD;
        let mode_offset: usize = HOSTFS_DATA_START + Self::OFFSET_OF_MODE;
        payload[fd_offset..fd_offset + Self::SIZE_OF_FD]
            .copy_from_slice(&self.fd.to_le_bytes());
        payload[mode_offset..mode_offset + Self::SIZE_OF_MODE]
            .copy_from_slice(&self.mode.to_le_bytes());
        payload
    }

    /// Decodes this request from a hostfs message payload.
    pub fn decode(payload: &[u8; Message::PAYLOAD_SIZE]) -> Self {
        let fd_offset: usize = HOSTFS_DATA_START + Self::OFFSET_OF_FD;
        let mode_offset: usize = HOSTFS_DATA_START + Self::OFFSET_OF_MODE;
        Self {
            fd: i32::from_le_bytes(
                payload[fd_offset..fd_offset + Self::SIZE_OF_FD]
                    .try_into()
                    .expect("file descriptor field size must match i32"),
            ),
            mode: u32::from_le_bytes(
                payload[mode_offset..mode_offset + Self::SIZE_OF_MODE]
                    .try_into()
                    .expect("mode field size must match u32"),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trip() {
        let request: ChmodRequest = ChmodRequest::new(17, 0o754);
        let payload: [u8; Message::PAYLOAD_SIZE] = request.serialize(123, OperationId::new(42));

        assert_eq!(ChmodRequest::decode(&payload).fd(), request.fd());
        assert_eq!(ChmodRequest::decode(&payload).mode(), request.mode());
        assert_eq!(crate::get_op_id(&payload), OperationId::new(42));
    }
}
