// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
