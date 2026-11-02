// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn terminal_signal_message_preserves_signum() {
    let message: TerminalSignalMessage = TerminalSignalMessage::new(2);
    let decoded: TerminalSignalMessage = TerminalSignalMessage::from_bytes(message.into_bytes());
    assert_eq!({ decoded.signum }, 2);
}

#[test]
fn terminal_access_message_preserves_fields() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(7);

    let read: TerminalAccessMessage = TerminalAccessMessage::new(pid, false);
    let read: TerminalAccessMessage = TerminalAccessMessage::from_bytes(read.into_bytes());
    assert_eq!({ read.pid }, pid);
    assert!(!read.is_write());

    let write: TerminalAccessMessage = TerminalAccessMessage::new(pid, true);
    let write: TerminalAccessMessage = TerminalAccessMessage::from_bytes(write.into_bytes());
    assert_eq!({ write.pid }, pid);
    assert!(write.is_write());
}
