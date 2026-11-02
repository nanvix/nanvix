// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests request field preservation through serialization.
#[test]
fn request_round_trip() {
    let request: PollSocketRequest = PollSocketRequest::new(7, 0x123);
    let decoded: PollSocketRequest = PollSocketRequest::from_bytes(request.into_bytes());
    assert_eq!(decoded.sockfd(), 7);
    assert_eq!(decoded.events(), 0x123);
}

/// Tests response field preservation through serialization.
#[test]
fn response_round_trip() {
    let response: PollSocketResponse = PollSocketResponse::new(0x45);
    let decoded: PollSocketResponse = PollSocketResponse::from_bytes(response.into_bytes());
    assert_eq!(decoded.revents(), 0x45);
}
