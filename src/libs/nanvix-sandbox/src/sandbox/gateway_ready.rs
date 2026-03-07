// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Shared logic for waiting on `GatewayReady` notifications from linuxd.
//!
//! Both `multi_process::LinuxDaemon` and `single_process::LinuxDaemon` delegate to the functions
//! in this module so the read-loop and buffering logic is defined exactly once.

use ::anyhow::Result;
use ::control_plane_api::{
    LinuxdCommand,
    LinuxdControlMessage,
};
use ::log::{
    debug,
    error,
    trace,
};
use ::std::collections::HashMap;
use ::syscomm::{
    ReadExact,
    SocketStream,
};
use ::tokio::time::{
    timeout,
    Duration,
    Instant,
};

///
/// # Description
///
/// Waits for a `GatewayReady` notification from linuxd on the control-plane stream. Non-matching
/// notifications are buffered in `pending` so that subsequent callers can find them.
///
/// # Parameters
///
/// - `stream`: Control-plane socket stream connected to linuxd.
/// - `pending`: Set of gateway IDs for which a `GatewayReady` was already received but not yet
///   claimed.
/// - `expected_gateway_id`: Identifier of the User VM whose `GatewayReady` is expected.
/// - `gateway_timeout`: Maximum duration to wait for the notification.
///
/// # Returns
///
/// On success, returns `Ok(())`. On failure or timeout, returns an error.
///
pub(crate) async fn wait_for_gateway_ready(
    stream: &mut SocketStream,
    pending: &mut HashMap<u32, usize>,
    expected_gateway_id: u32,
    gateway_timeout: Duration,
) -> Result<()> {
    trace!("wait_for_gateway_ready(expected_gateway_id={expected_gateway_id})");

    let deadline = Instant::now() + gateway_timeout;

    // Check if the notification was already buffered by a previous call.
    if let Some(count) = pending.get_mut(&expected_gateway_id) {
        if *count > 1 {
            *count -= 1;
        } else {
            pending.remove(&expected_gateway_id);
        }
        debug!("found buffered GatewayReady from linuxd (gateway_id={expected_gateway_id})");
        return Ok(());
    }

    loop {
        let mut buffer: [u8; LinuxdControlMessage::WIRE_SIZE] =
            [0u8; LinuxdControlMessage::WIRE_SIZE];

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let reason: String = format!(
                "timed out waiting for GatewayReady from linuxd \
                 (expected_gateway_id={expected_gateway_id}, timeout={gateway_timeout:?})"
            );
            error!("wait_for_gateway_ready(): {reason}");
            anyhow::bail!(reason);
        }

        match timeout(remaining, stream.read_exact(&mut buffer)).await {
            Ok(Ok(_n)) => {},
            Ok(Err(e)) => {
                let reason: String =
                    format!("failed reading GatewayReady from control-plane (error={e:?})");
                error!("wait_for_gateway_ready(): {reason}");
                anyhow::bail!(reason);
            },
            Err(_) => {
                let reason: String = format!(
                    "timed out waiting for GatewayReady from linuxd \
                     (expected_gateway_id={expected_gateway_id}, timeout={gateway_timeout:?})"
                );
                error!("wait_for_gateway_ready(): {reason}");
                anyhow::bail!(reason);
            },
        }

        let msg: LinuxdControlMessage =
            LinuxdControlMessage::try_from_bytes(&buffer).map_err(|e| {
                let reason: String =
                    format!("invalid control-plane message from linuxd (error={e:?})");
                error!("wait_for_gateway_ready(): {reason}");
                anyhow::anyhow!(reason)
            })?;

        match msg.cmd() {
            LinuxdCommand::GatewayReady => {
                if msg.gateway_id() == expected_gateway_id {
                    debug!(
                        "received matching GatewayReady from linuxd \
                         (gateway_id={expected_gateway_id})"
                    );
                    return Ok(());
                }
                // Buffer the notification so the rightful caller finds it.
                *pending.entry(msg.gateway_id()).or_insert(0) += 1;
                trace!(
                    "buffered GatewayReady for different VM (received={}, \
                     expected={expected_gateway_id})",
                    msg.gateway_id()
                );
            },
            #[allow(unreachable_patterns)]
            other => {
                let reason: String = format!(
                    "unexpected linuxd command while waiting for GatewayReady (cmd={other:?})"
                );
                error!("wait_for_gateway_ready(): {reason}");
                anyhow::bail!(reason)
            },
        }
    }
}

/// Reproduces the old buggy behavior that discards non-matching `GatewayReady` messages instead of
/// buffering them. Used only by regression tests to prove the fix is necessary.
#[cfg(test)]
pub(crate) async fn wait_for_gateway_ready_no_buffer(
    stream: &mut SocketStream,
    expected_gateway_id: u32,
    gateway_timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + gateway_timeout;

    loop {
        let mut buffer: [u8; LinuxdControlMessage::WIRE_SIZE] =
            [0u8; LinuxdControlMessage::WIRE_SIZE];

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            anyhow::bail!("timed out (no buffer)");
        }

        match timeout(remaining, stream.read_exact(&mut buffer)).await {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => anyhow::bail!("read error: {e:?}"),
            Err(_) => anyhow::bail!("timed out (no buffer)"),
        }

        let msg: LinuxdControlMessage = LinuxdControlMessage::try_from_bytes(&buffer)
            .map_err(|e| anyhow::anyhow!("invalid message: {e:?}"))?;

        if msg.cmd() == LinuxdCommand::GatewayReady && msg.gateway_id() == expected_gateway_id {
            return Ok(());
        }
        // BUG: non-matching messages are silently discarded.
    }
}
