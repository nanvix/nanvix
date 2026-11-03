// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Returns the number of variants of [`SystemCallMessageKind`].
fn system_call_message_kind_count() -> u16 {
    u16::try_from(mem::variant_count::<SystemCallMessageKind>())
        .expect("number of message kinds should fit in a wire value")
}

/// Wire values are assigned contiguously from zero, so every value below the number of variants
/// must decode to the variant that it identifies. Because these values are distinct, this also
/// checks that the decoder does not miss any variant.
#[test]
fn system_call_message_kind_wire_values_round_trip() {
    for value in 0..system_call_message_kind_count() {
        let kind: SystemCallMessageKind = SystemCallMessageKind::try_from(value)
            .unwrap_or_else(|()| panic!("wire value {value} should decode"));
        assert_eq!(kind as u16, value, "wire value {value} decodes to {kind:?}");
    }
}

/// Values at or above the number of variants are not assigned to any variant, so the decoder
/// must reject them.
#[test]
fn system_call_message_kind_rejects_unassigned_wire_values() {
    for value in system_call_message_kind_count()..=u16::MAX {
        assert!(
            SystemCallMessageKind::try_from(value).is_err(),
            "unassigned wire value {value} should not decode"
        );
    }
}

#[test]
fn system_call_message_request_id_round_trip() {
    let request_id: RequestIdentifier = RequestIdentifier::from_raw(0x1234_5678);
    let message: SystemCallMessage = SystemCallMessage::new(
        SystemCallMessageKind::CloseRequest,
        [0; SystemCallMessage::PAYLOAD_SIZE],
    );
    let bytes: [u8; Message::PAYLOAD_SIZE] = message.into_bytes();
    let mut outer: Message = Message::new(
        ::sys::ipc::MessageSender::KERNEL,
        ::sys::ipc::MessageReceiver::KERNEL,
        ::sys::ipc::MessageType::Ipc,
        None,
        bytes,
    );
    request_id.write_to(&mut outer);
    assert_eq!(
        &outer.payload
            [REQUEST_IDENTIFIER_OFFSET..REQUEST_IDENTIFIER_OFFSET + RequestIdentifier::SIZE],
        &request_id.raw().to_ne_bytes()
    );
    assert_eq!(RequestIdentifier::read_from(&outer), request_id);

    let decoded: SystemCallMessage =
        SystemCallMessage::try_from_bytes(outer.payload).expect("message should decode");
    assert_eq!(decoded.kind(), SystemCallMessageKind::CloseRequest);
    assert_eq!(decoded.request_id(), request_id);
}
