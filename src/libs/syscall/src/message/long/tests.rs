// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

fn part(total_parts: u16, part_number: u16, payload_size: u8) -> SystemCallMessagePart {
    SystemCallMessagePart {
        total_parts,
        part_number,
        payload_size,
        payload: [0; SystemCallMessagePart::PAYLOAD_SIZE],
    }
}

#[test]
fn rejects_oversized_part_payload() {
    let mut message: SystemCallLongMessage =
        SystemCallLongMessage::new(1).expect("capacity should be valid");
    let oversized: u8 = (SystemCallMessagePart::PAYLOAD_SIZE + 1) as u8;

    let error: Error = message
        .add_part(part(1, 0, oversized))
        .expect_err("oversized payload should be rejected");
    assert_eq!(error.code, ErrorCode::InvalidMessage);
}

#[test]
fn rejects_duplicate_part_number() {
    let mut message: SystemCallLongMessage =
        SystemCallLongMessage::new(2).expect("capacity should be valid");
    message
        .add_part(part(2, 0, 1))
        .expect("first part should be accepted");

    let error: Error = message
        .add_part(part(2, 0, 1))
        .expect_err("duplicate part should be rejected");
    assert_eq!(error.code, ErrorCode::InvalidMessage);
}
