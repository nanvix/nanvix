// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn request_round_trip() {
    let request: ChmodRequest = ChmodRequest::new(17, 0o754);
    let payload: [u8; Message::PAYLOAD_SIZE] = request.serialize(123, OperationId::new(42));

    assert_eq!(ChmodRequest::decode(&payload).fd(), request.fd());
    assert_eq!(ChmodRequest::decode(&payload).mode(), request.mode());
    assert_eq!(crate::get_op_id(&payload), OperationId::new(42));
}
