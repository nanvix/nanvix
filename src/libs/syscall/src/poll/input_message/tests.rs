// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests request field preservation through serialization.
#[test]
fn request_round_trip() {
    let request: PollInputRequest = PollInputRequest::new(256);
    let decoded: PollInputRequest = PollInputRequest::from_bytes(request.into_bytes());
    assert_eq!(decoded.count(), 256);
}

/// Tests response field preservation through serialization.
#[test]
fn response_round_trip() {
    let response: PollInputResponse = PollInputResponse::new(PollInputRequest::STATUS_DATA);
    let decoded: PollInputResponse = PollInputResponse::from_bytes(response.into_bytes());
    assert_eq!(decoded.status(), PollInputRequest::STATUS_DATA);
}

/// Tests console-read cancellation target preservation through serialization.
#[test]
fn console_read_cancel_request_round_trip() {
    let target: RequestIdentifier = RequestIdentifier::from_raw(0x1234_5678);
    let request: Message = ConsoleReadCancel::build_request(ThreadIdentifier::from(1), target);
    let message: SystemCallMessage = SystemCallMessage::try_from_bytes(request.payload)
        .expect("console cancellation request should decode");
    assert_eq!(ConsoleReadCancel::target(&message.payload), target);
}

/// Tests console-read cancellation outcome preservation through serialization.
#[test]
fn console_read_cancel_response_round_trip() {
    for cancelled in [false, true] {
        let response: Message =
            ConsoleReadCancel::build_response(ThreadIdentifier::from(1), cancelled);
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)
            .expect("console cancellation response should decode");
        assert_eq!(ConsoleReadCancel::cancelled(&message.payload), cancelled);
    }
}

/// Tests pipe cancellation request field preservation through serialization.
#[test]
fn pipe_cancel_request_round_trip() {
    let request_id: RequestIdentifier = RequestIdentifier::from_raw(42);
    let request: PipeOpCancelRequest =
        PipeOpCancelRequest::new(7, PipeOperation::Write, request_id);
    let decoded: PipeOpCancelRequest = PipeOpCancelRequest::from_bytes(request.into_bytes());
    assert_eq!(decoded.fd(), 7);
    assert_eq!(decoded.operation(), Some(PipeOperation::Write));
    assert_eq!(decoded.request_id(), request_id);
}

/// Tests pipe cancellation response field preservation through serialization.
#[test]
fn pipe_cancel_response_round_trip() {
    let response: PipeOpCancelResponse = PipeOpCancelResponse::new(123, true);
    let decoded: PipeOpCancelResponse = PipeOpCancelResponse::from_bytes(response.into_bytes());
    assert_eq!(decoded.transferred(), 123);
    assert!(decoded.cancelled());
}
