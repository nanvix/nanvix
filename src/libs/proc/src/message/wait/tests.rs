// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn wait_cancel_request_round_trip() {
    let request_id: RequestIdentifier = RequestIdentifier::from_raw(42);
    let request: WaitCancelMessage = WaitCancelMessage::new(request_id);
    let decoded: WaitCancelMessage = WaitCancelMessage::from_bytes(request.into_bytes());
    assert_eq!(decoded.request_id(), request_id);
}

#[test]
fn wait_cancel_response_round_trip() {
    let response: WaitCancelResponseMessage = WaitCancelResponseMessage::new(true);
    let decoded: WaitCancelResponseMessage =
        WaitCancelResponseMessage::from_bytes(response.into_bytes());
    assert!(decoded.cancelled());
}
