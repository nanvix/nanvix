// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn long_message_size_boundary() {
    assert!(is_valid_long_message_size(MAX_LONG_MESSAGE_SIZE));
    assert!(!is_valid_long_message_size(MAX_LONG_MESSAGE_SIZE + 1));
    assert!(!is_valid_long_message_size(0));
}

#[test]
#[cfg(feature = "std")]
fn long_link_round_trip() {
    let op_id: OperationId = OperationId::new(11);
    let bytes: Vec<u8> = serialize_long_link_request(op_id, 0x400, b"old/path", b"new/path")
        .expect("link request should serialize");
    let request: LongLinkRequest =
        deserialize_long_link(&bytes).expect("link request should deserialize");

    assert_eq!(request.op_id, op_id);
    assert_eq!(request.flags, 0x400);
    assert_eq!(request.old_path, "old/path");
    assert_eq!(request.new_path, "new/path");
}

#[test]
fn mode_path_request_round_trip() {
    let bytes = serialize_long_mode_path_request(OperationId::new(9), 0o754, 2, b"dir/file")
        .expect("mode/path request should serialize");
    #[cfg(feature = "std")]
    {
        let request = deserialize_long_mode_path(&bytes).expect("request should deserialize");
        assert_eq!(request.op_id(), OperationId::new(9));
        assert_eq!(request.mode(), 0o754);
        assert_eq!(request.flags(), 2);
        assert_eq!(request.path(), "dir/file");
    }
}

#[test]
fn long_response_part_enforces_chunk_limit() {
    let chunk: [u8; LONG_RESPONSE_CHUNK_SIZE] = [0x5a; LONG_RESPONSE_CHUNK_SIZE];
    let part: [u8; Message::PAYLOAD_SIZE] =
        build_long_response_part(7, OperationId::new(11), 1, 0, &chunk)
            .expect("maximum-sized chunk should fit");
    assert_eq!(part[10] as usize, LONG_RESPONSE_CHUNK_SIZE);
    assert_eq!(&part[11..], &chunk);

    let oversized: [u8; LONG_RESPONSE_CHUNK_SIZE + 1] = [0; LONG_RESPONSE_CHUNK_SIZE + 1];
    assert!(build_long_response_part(7, OperationId::new(11), 1, 0, &oversized).is_none());
}
