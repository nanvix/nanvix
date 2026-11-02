// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::sys::ipc::{
    Message,
    MessageReceiver,
    MessageSender,
    MessageType,
    RequestIdentifier,
    SystemMessageHeader,
};

#[test]
fn process_management_request_id_uses_shared_raw_offset() {
    let request_id: u32 = 0x8765_4321;
    let mut process: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::Wait,
        [0; ProcessManagementMessage::PAYLOAD_SIZE],
    );
    process.request_id = request_id;
    let system: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, process.into_bytes());
    let bytes: [u8; Message::PAYLOAD_SIZE] = system.into_bytes();
    assert_eq!(
        &bytes[REQUEST_IDENTIFIER_OFFSET..REQUEST_IDENTIFIER_OFFSET + RequestIdentifier::SIZE],
        &request_id.to_ne_bytes()
    );

    let mut outer: Message =
        Message::new(MessageSender::KERNEL, MessageReceiver::KERNEL, MessageType::Ipc, None, bytes);
    assert_eq!(RequestIdentifier::read_from(&outer).raw(), request_id);

    let replacement: RequestIdentifier = RequestIdentifier::from_raw(0x1234_5678);
    replacement.write_to(&mut outer);

    let system: SystemMessage =
        SystemMessage::from_bytes(outer.payload).expect("system message should decode");
    let decoded: ProcessManagementMessage = ProcessManagementMessage::from_bytes(system.payload)
        .expect("process message should decode");
    let decoded_request_id: u32 = decoded.request_id;
    assert_eq!(decoded_request_id, replacement.raw());
}
