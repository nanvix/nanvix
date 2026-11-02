// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn terminal_detach_request_round_trip() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(42);
    let request: TerminalDetachMessage = TerminalDetachMessage::new(pid);
    let decoded: TerminalDetachMessage = TerminalDetachMessage::from_bytes(request.into_bytes());

    assert_eq!({ decoded.pid }, pid);
}

#[test]
fn terminal_detach_ack_round_trip() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(42);
    let status: i32 = -1;
    let acknowledgement: TerminalDetachAckMessage = TerminalDetachAckMessage::new(pid, status);
    let decoded: TerminalDetachAckMessage =
        TerminalDetachAckMessage::from_bytes(acknowledgement.into_bytes());

    assert_eq!({ decoded.pid }, pid);
    assert_eq!({ decoded.status }, status);
}
