// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

fn part(total_parts: u16, part_number: u16) -> SystemCallMessagePart {
    SystemCallMessagePart {
        total_parts,
        part_number,
        payload_size: SystemCallMessagePart::PAYLOAD_SIZE as u8,
        payload: [0; SystemCallMessagePart::PAYLOAD_SIZE],
    }
}

#[test]
fn assembler_size_boundary() {
    assert_eq!(MAX_LONG_BUFFER_SIZE % SystemCallMessagePart::PAYLOAD_SIZE, 0);

    let mut assembler: HostFsAssembler = HostFsAssembler::new();
    for part_number in 0..MAX_TOTAL_PARTS {
        let expected: AssemblyStatus = if part_number + 1 == MAX_TOTAL_PARTS {
            AssemblyStatus::Complete
        } else {
            AssemblyStatus::NeedMore
        };
        assert_eq!(
            assembler.add_part(
                SystemCallMessageKind::HostFsOpenRequestPart,
                OperationId::from_raw(1),
                part(MAX_TOTAL_PARTS, part_number),
            ),
            expected
        );
    }
    assert_eq!(assembler.buffer.len(), MAX_LONG_BUFFER_SIZE);

    let mut assembler: HostFsAssembler = HostFsAssembler::new();
    assert_eq!(
        assembler.add_part(
            SystemCallMessageKind::HostFsOpenRequestPart,
            OperationId::from_raw(1),
            part(MAX_TOTAL_PARTS + 1, 0),
        ),
        AssemblyStatus::Error
    );
}
