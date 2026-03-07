// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Shared tests for `wait_for_gateway_ready` and `wait_for_gateway_ready_no_buffer`.
//!
//! Both `multi_process::LinuxDaemon` and `single_process::LinuxDaemon` expose identical testing
//! interfaces (`new_for_test`, `wait_for_gateway_ready`, `wait_for_gateway_ready_no_buffer`) so
//! the test logic is defined once here and included from each `linuxd` module via `#[path]`.

use super::*;
use ::control_plane_api::{
    LinuxdCommand,
    LinuxdControlMessage,
};
use ::syscomm::WriteAll;
use ::tokio::time::Duration;

#[allow(clippy::expect_used)]
/// Helper: write a `GatewayReady` message for the given `gateway_id` into the stream.
async fn write_gateway_ready(stream: &mut SocketStream, gateway_id: u32) {
    let msg: LinuxdControlMessage =
        LinuxdControlMessage::new(LinuxdCommand::GatewayReady, gateway_id);
    let mut buf: [u8; LinuxdControlMessage::WIRE_SIZE] = [0u8; LinuxdControlMessage::WIRE_SIZE];
    msg.to_bytes(&mut buf);
    stream
        .write_all(&buf)
        .await
        .expect("failed to write GatewayReady");
}

#[allow(clippy::expect_used)]
/// Helper: create a Unix socket pair wrapped in `SocketStream`.
fn socket_pair() -> (SocketStream, SocketStream) {
    let (a, b) = ::tokio::net::UnixStream::pair().expect("failed to create socket pair");
    (SocketStream::Unix(a), SocketStream::Unix(b))
}

/// Verify that `wait_for_gateway_ready` returns immediately when the expected message arrives
/// first.
#[tokio::test]
async fn gateway_ready_direct_match() {
    let (reader, mut writer) = socket_pair();
    let daemon: LinuxDaemon = LinuxDaemon::new_for_test(reader);

    write_gateway_ready(&mut writer, 0).await;

    daemon
        .wait_for_gateway_ready(0, Duration::from_secs(5))
        .await
        .expect("expected direct match to succeed");
}

/// Verify that messages for other gateway IDs are buffered and the expected one is still found.
#[tokio::test]
async fn gateway_ready_buffers_other_ids() {
    let (reader, mut writer) = socket_pair();
    let daemon: LinuxDaemon = LinuxDaemon::new_for_test(reader);

    // Send GatewayReady for id=1 first, then id=0.
    write_gateway_ready(&mut writer, 1).await;
    write_gateway_ready(&mut writer, 0).await;

    // Wait for id=0 — should consume id=1 into the buffer and then match id=0.
    daemon
        .wait_for_gateway_ready(0, Duration::from_secs(5))
        .await
        .expect("expected buffered match to succeed");

    // Now wait for id=1 — should be found in the buffer immediately.
    daemon
        .wait_for_gateway_ready(1, Duration::from_secs(5))
        .await
        .expect("expected id=1 to be in the buffer");
}

/// Verify that multiple out-of-order messages are all buffered correctly.
#[tokio::test]
async fn gateway_ready_multiple_out_of_order() {
    let (reader, mut writer) = socket_pair();
    let daemon: LinuxDaemon = LinuxDaemon::new_for_test(reader);

    // Send messages in reverse order: 3, 2, 1, 0.
    for id in (0u32..4).rev() {
        write_gateway_ready(&mut writer, id).await;
    }

    // Request id=0 — must read through 3, 2, 1 (buffering them) before finding 0.
    daemon
        .wait_for_gateway_ready(0, Duration::from_secs(5))
        .await
        .expect("expected id=0 to succeed");

    // The remaining IDs should all be in the buffer.
    for id in 1u32..4 {
        daemon
            .wait_for_gateway_ready(id, Duration::from_secs(5))
            .await
            .unwrap_or_else(|_| panic!("expected id={id} to be in the buffer"));
    }
}

/// Verify that a timeout is returned when no matching message arrives.
#[tokio::test]
async fn gateway_ready_timeout() {
    let (reader, _writer) = socket_pair();
    let daemon: LinuxDaemon = LinuxDaemon::new_for_test(reader);

    let result = daemon
        .wait_for_gateway_ready(42, Duration::from_millis(50))
        .await;

    assert!(result.is_err(), "expected timeout error");
}

/// Regression: without the buffering fix, the second caller's message is lost and times out.
#[tokio::test]
async fn regression_without_fix_second_call_times_out() {
    let (reader, mut writer) = socket_pair();
    let daemon: LinuxDaemon = LinuxDaemon::new_for_test(reader);

    // Send id=1 then id=0.
    write_gateway_ready(&mut writer, 1).await;
    write_gateway_ready(&mut writer, 0).await;

    // First call reads and discards id=1, then matches id=0.
    daemon
        .wait_for_gateway_ready_no_buffer(0, Duration::from_secs(5))
        .await
        .expect("first call should succeed even without buffering");

    // Second call times out because id=1 was discarded.
    let result = daemon
        .wait_for_gateway_ready_no_buffer(1, Duration::from_millis(100))
        .await;
    assert!(
        result.is_err(),
        "without the fix, id=1 was discarded and the second call must time out"
    );
}

/// Regression: with the buffering fix, the same scenario succeeds for both callers.
#[tokio::test]
async fn regression_with_fix_second_call_succeeds() {
    let (reader, mut writer) = socket_pair();
    let daemon: LinuxDaemon = LinuxDaemon::new_for_test(reader);

    // Same message order as the regression test above.
    write_gateway_ready(&mut writer, 1).await;
    write_gateway_ready(&mut writer, 0).await;

    // First call buffers id=1, then matches id=0.
    daemon
        .wait_for_gateway_ready(0, Duration::from_secs(5))
        .await
        .expect("first call should succeed");

    // Second call finds id=1 in the buffer.
    daemon
        .wait_for_gateway_ready(1, Duration::from_secs(5))
        .await
        .expect("with the fix, id=1 should be found in the buffer");
}
