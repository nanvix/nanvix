// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::alloc::format;

#[test]
fn message_type_discriminants_round_trip() {
    let cases: [(MessageType, u8); 8] = [
        (MessageType::Interrupt, 1),
        (MessageType::Exception, 2),
        (MessageType::Ipc, 3),
        (MessageType::ProcessTerminationEvent, 4),
        (MessageType::Ikc, 5),
        (MessageType::PullResponse, 6),
        (MessageType::ProcessCreationEvent, 7),
        (MessageType::ThreadTerminationEvent, 8),
    ];

    for (message_type, raw) in cases {
        assert_eq!(message_type.to_bytes(), [raw]);
        assert_eq!(
            MessageType::try_from_bytes([raw]).expect("valid message type should parse"),
            message_type
        );
    }
}

#[test]
fn invalid_message_type_discriminants_are_rejected() {
    assert!(MessageType::try_from_bytes([0]).is_err());
    assert!(MessageType::try_from_bytes([9]).is_err());
    assert!(MessageType::try_from_bytes([u8::MAX]).is_err());
}

#[test]
fn thread_termination_message_type_is_formatted() {
    assert_eq!(format!("{:?}", MessageType::ThreadTerminationEvent), "thread termination event");
}
