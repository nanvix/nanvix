// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # IPC Source-Stamping Regression Tests
//!
//! Regression coverage for the kernel IPC `send()` path, which stamps the authoritative identity
//! of the calling process and thread into `message.source` (overwriting whatever the sender
//! supplied) so that a receiver can trust `message.source.pid` for request attribution.
//!
//! Before this guarantee the kernel only logged a warning when it detected a forged `source` and
//! still delivered the message with the attacker-supplied value intact. Any receiver that trusted
//! `message.source` for request attribution could then be tricked into performing privileged
//! actions on the sender's behalf. This is a broken-access-control / source-spoofing vulnerability
//! (OWASP A01).
//!
//! The tests below verify that:
//!
//! 1. A send whose `source` is forged to impersonate a different privileged daemon is delivered
//!    with the forged identity overwritten by the caller's true [`ProcessIdentifier`], so the
//!    forged value never reaches the receiver.
//! 2. A send carrying the caller's own legitimate `source` still round-trips with the caller's
//!    identity intact, ensuring the stamping does not corrupt well-formed messages.

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
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Test Cases
//==================================================================================================

///
/// # Description
///
/// Verifies that the kernel overwrites a forged `source` with the caller's true identity.
///
/// The forged source impersonates a different privileged daemon. A correct kernel stamps the
/// caller's authoritative [`ProcessIdentifier`] over it, so the delivered message must carry the
/// caller's own pid — never the forged one.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
/// # Errors
///
/// This function returns an error if the message cannot be sent or received, or if the kernel does
/// not stamp the caller's authoritative process identifier on the message.
///
/// # Panics
///
/// This function panics if the forged process identifier unexpectedly equals the caller's process
/// identifier.
///
fn test_spoofed_source_is_overwritten() -> Result<(), Error> {
    let my_pid: ProcessIdentifier = pm::getpid_uncached()?;

    // Impersonate a privileged daemon other than ourselves. The test image includes procd as the
    // lifecycle consumer, so the test process normally forges VFSD.
    let forged_pid: ProcessIdentifier = if my_pid == ProcessIdentifier::VFSD {
        ProcessIdentifier::PROCD
    } else {
        ProcessIdentifier::VFSD
    };

    // Guard against the spoof silently becoming a no-op if the environment ever changes.
    assert!(forged_pid != my_pid, "forged source must differ from the caller's own identity");

    // The message is addressed to ourselves so that delivery has no external side effects.
    let spoofed: Message = Message::new(
        MessageSender::new(forged_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(my_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        [0u8; Message::PAYLOAD_SIZE],
    );

    // The send succeeds: the kernel does not reject the forged source, it overwrites it.
    ipc::__kcall_send(&spoofed)?;

    // Drain the delivered message and confirm the forged identity did not survive.
    let received: Message = ipc::__kcall_recv()?;
    let source = { received.source };
    if source.pid == forged_pid {
        return Err(Error::new(
            ErrorCode::OperationNotPermitted,
            "kernel delivered a spoofed message source",
        ));
    }
    if source.pid != my_pid {
        return Err(Error::new(
            ErrorCode::OperationNotPermitted,
            "kernel stamped an unexpected source identity",
        ));
    }

    Ok(())
}

/// Verifies that a send carrying the caller's legitimate source still round-trips with the
/// caller's identity intact, ensuring the authoritative stamping does not corrupt well-formed
/// messages.
fn test_allow_legitimate_source() -> Result<(), Error> {
    let my_pid: ProcessIdentifier = pm::getpid_uncached()?;

    // A message from ourselves to ourselves carries a legitimate source and must be accepted.
    const PAYLOAD_MARKER: u8 = 0xA5;
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    payload[0] = PAYLOAD_MARKER;
    let legit: Message = Message::new(
        MessageSender::new(my_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(my_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&legit)?;

    // Drain the delivered message so it does not leak into later tests, and confirm that the
    // kernel-stamped source identifies the caller and the payload survived the round trip.
    let received: Message = ipc::__kcall_recv()?;
    let received_source = { received.source };
    let received_payload: [u8; Message::PAYLOAD_SIZE] = { received.payload };
    assert!(received_source.pid == my_pid, "legitimate message source was altered in transit");
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

    test_spoofed_source_is_overwritten()?;
    ::syslog::info!("test-kernel: source_spoofing: PASS - spoofed_source_is_overwritten");

    test_allow_legitimate_source()?;
    ::syslog::info!("test-kernel: source_spoofing: PASS - allow_legitimate_source");

    ::syslog::info!("test-kernel: source_spoofing: all tests passed");

    Ok(())
}
