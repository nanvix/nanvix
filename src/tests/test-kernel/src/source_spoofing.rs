// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # IPC Source-Spoofing Regression Tests
//!
//! Regression coverage for the kernel IPC `send()` path, which must reject messages whose
//! `source` field does not match the trusted identity of the calling process or thread.
//!
//! Before the fix the kernel only logged a warning when it detected a forged `source` and still
//! delivered the message with the attacker-supplied value intact. Because daemons authenticate
//! callers on `message.source` (e.g. vfsd treats a message as coming from `PROCD`), any
//! unprivileged process could impersonate a privileged peer and trick a receiver into performing
//! privileged actions on its behalf. This is a broken-access-control / source-spoofing
//! vulnerability (OWASP A01).
//!
//! The tests below verify that:
//!
//! 1. A send whose `source` is forged to impersonate a different privileged daemon (one whose
//!    identity matches neither the caller's PID-encoded identity nor its negative TID-encoded
//!    identity) is rejected with [`ErrorCode::OperationNotPermitted`] and is therefore not
//!    delivered.
//! 2. A send carrying the caller's own legitimate PID-encoded `source` still succeeds and is
//!    delivered intact, ensuring the rejection does not break well-formed messages.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    kcall::{
        ipc,
        pm,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Test Cases
//==================================================================================================

/// Verifies that the kernel rejects a send whose `source` is forged to impersonate another process.
///
/// The forged source is chosen to match neither the caller's PID-encoded identity nor its
/// (negative) TID-encoded identity, so the kernel must reject the send with
/// [`ErrorCode::OperationNotPermitted`] instead of delivering the forged message.
fn test_reject_spoofed_source() -> Result<(), Error> {
    let my_pid: ProcessIdentifier = pm::__kcall_getpid()?;

    // Impersonate a privileged daemon other than ourselves. In the test-kernel environment this
    // process runs as PROCD, so forge VFSD; if it ever runs as VFSD, forge PROCD instead. Either
    // way the forged source matches neither our PID-encoded identity nor our (negative)
    // TID-encoded identity, so a correct kernel must reject it.
    let forged_pid: ProcessIdentifier = if my_pid == ProcessIdentifier::VFSD {
        ProcessIdentifier::PROCD
    } else {
        ProcessIdentifier::VFSD
    };

    // Guard against the spoof silently becoming a no-op if the environment ever changes.
    assert!(forged_pid != my_pid, "forged source must differ from the caller's own identity");

    // The message is addressed to ourselves so that the buggy delivery path (which would otherwise
    // reach a real daemon) has no external side effects.
    let spoofed: Message = Message::new(
        MessageSender::from(forged_pid),
        MessageReceiver::from(my_pid),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );

    match ipc::__kcall_send(&spoofed) {
        // The kernel accepted the forged source. This is the bug under test: the send must be
        // rejected, not delivered.
        Ok(()) => Err(Error::new(
            ErrorCode::OperationNotPermitted,
            "kernel accepted a spoofed message source",
        )),
        // The kernel rejected the forged source as expected.
        Err(e) if e.code == ErrorCode::OperationNotPermitted => Ok(()),
        // The send failed for an unexpected reason; surface it so the failure is not masked.
        Err(e) => Err(e),
    }
}

/// Verifies that a send carrying the caller's legitimate PID-encoded source still succeeds and is
/// delivered intact, ensuring the spoof rejection does not reject well-formed messages.
fn test_allow_legitimate_source() -> Result<(), Error> {
    let my_pid: ProcessIdentifier = pm::__kcall_getpid()?;

    // A message from ourselves to ourselves carries a legitimate source and must be accepted.
    const PAYLOAD_MARKER: u8 = 0xA5;
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0] = PAYLOAD_MARKER;
    let legit: Message = Message::new(
        MessageSender::from(my_pid),
        MessageReceiver::from(my_pid),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&legit)?;

    // Drain the delivered message so it does not leak into later tests, and confirm that the
    // legitimate source and payload survived the round trip unchanged.
    let received: Message = ipc::__kcall_recv()?;
    let received_source: MessageSender = { received.source };
    let received_payload: [u8; Message::PAYLOAD_SIZE] = { received.payload };
    assert!(
        received_source == MessageSender::from(my_pid),
        "legitimate message source was altered in transit"
    );
    assert!(
        received_payload[0] == PAYLOAD_MARKER,
        "legitimate message payload corrupted in transit"
    );

    Ok(())
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs all IPC source-spoofing regression tests.
pub fn run() -> Result<(), Error> {
    ::syslog::info!("test-kernel: source_spoofing: starting source-spoofing regression tests");

    test_reject_spoofed_source()?;
    ::syslog::info!("test-kernel: source_spoofing: PASS - reject_spoofed_source");

    test_allow_legitimate_source()?;
    ::syslog::info!("test-kernel: source_spoofing: PASS - allow_legitimate_source");

    ::syslog::info!("test-kernel: source_spoofing: all tests passed");

    Ok(())
}
