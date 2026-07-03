// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Linux Daemon management for multi-process mode.
//!
//! This module provides functionality to spawn and manage Linux Daemon instances as separate
//! processes. It handles process lifecycle, control-plane communication, and native execution.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::SHUTDOWN_TIMEOUT,
    LinuxDaemonArgs,
};
use ::anyhow::Result;
use ::control_plane_api::{
    NanvixdCommand,
    NanvixdControlMessage,
};
use ::linuxd::args;
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::std::{
    collections::HashMap,
    io::ErrorKind,
    process::Stdio,
};
use ::syscomm::{
    SocketStream,
    WriteAll,
};
use ::tokio::{
    process::{
        Child,
        Command,
    },
    sync::Mutex,
    time::{
        timeout,
        Duration,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Interior mutable state for a Linux Daemon instance.
///
struct LinuxDaemonInner {
    /// Child process handle.
    child: Child,
    /// Control-plane socket stream.
    control_plane_stream: SocketStream,
    /// Set of gateway IDs for which a `GatewayReady` notification has already been received but not
    /// yet claimed by the corresponding caller.
    pending_gateway_ready: HashMap<u32, usize>,
}

/// # Description
///
/// Handle to a running Linux Daemon instance spawned as a separate process.
///
pub struct LinuxDaemon {
    /// Interior mutable state.
    inner: Mutex<Option<LinuxDaemonInner>>,
}

pub struct PendingLinuxDaemon {
    child: Child,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    ///
    /// # Description
    ///
    /// Helper method to send a SIGKILL to the linuxd process in case it is faulty and we need to
    /// clean-up.
    ///
    /// # Parameters
    ///
    /// - `child`: The linuxd process handle.
    ///
    fn send_sigkill_to_child(child: &Child) {
        if let Some(pid) = child.id() {
            debug!("killing linuxd instance (pid={pid:?})");
            let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        }
    }

    ///
    /// # Description
    ///
    /// Spawns a new Linux Daemon instance as a separate process.
    ///
    /// # Parameters
    ///
    /// - `args`: Linux Daemon arguments.
    ///
    /// # Returns
    ///
    /// On success, this function returns a handle to the spawned Linux Daemon instance. On failure,
    /// this function returns an error object instead.
    ///
    pub async fn spawn<T: Sync + Send + 'static>(
        args: &LinuxDaemonArgs<T>,
    ) -> Result<PendingLinuxDaemon> {
        debug!(
            "spawn(): spawning linux daemon (control-plane={:?}, user-vm={:?})",
            args.control_plane_connect_socket_info(),
            args.system_vm_socket_info(),
        );

        let mut linuxd_args: Vec<String> = vec![
            args.linuxd_binary_path().to_string(),
            args::Args::OPT_TENANT_ID.to_string(),
            args.tenant_id().to_string(),
            args::Args::OPT_LOGFILE.to_string(),
            args::Args::OPT_LOGDIR.to_string(),
            args.log_directory().to_string(),
            args::Args::OPT_CONTROL_PLANE_SOCKADDR.to_string(),
            args.control_plane_connect_socket_info().0.clone(),
            args::Args::OPT_CONTROL_PLANE_SOCKET_TYPE.to_string(),
            args.control_plane_connect_socket_info()
                .1
                .to_str()
                .to_string(),
            args::Args::OPT_USER_VM_BIND_SOCKADDR.to_string(),
            args.system_vm_socket_info().0.clone(),
            args::Args::OPT_USER_VM_BIND_SOCKET_TYPE.to_string(),
            args.system_vm_socket_info().1.to_str().to_string(),
        ];
        if args.networking_enabled() {
            linuxd_args.push(args::Args::OPT_NETWORKING_ENABLED.to_string());
        }
        if let Some(hwloc) = args.hwloc() {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                hwloc.get_linuxd_core_str(),
            ];
            linuxd_args.splice(0..0, taskset);
        }
        debug!("spawn(): spawning linuxd with args: {}", linuxd_args.join(" "));

        // Inherit stdout/stderr so that errors when spawning the command are surfaced to nanvixd.
        let child: Child = {
            let mut cmd: Command = Command::new(&linuxd_args[0]);
            cmd.args(&linuxd_args[1..]);
            // Ensure the child process is killed if the Child handle is dropped without explicit
            // cleanup. This acts as a best-effort safety net during normal unwinding and shutdown
            // paths where drop handlers run, helping to prevent orphaned processes.
            cmd.kill_on_drop(true);
            cmd.stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| {
                    let reason: String = format!("error spawning linuxd process (error={e:?})");
                    error!("spawn(): {reason}");
                    anyhow::anyhow!(reason)
                })?
        };

        Ok(PendingLinuxDaemon { child })
    }

    ///
    /// # Description
    ///
    /// Waits for a `GatewayReady` notification from linuxd on the control-plane stream. This
    /// replaces the previous busy-poll mechanism and provides event-driven synchronization.
    ///
    /// # Parameters
    ///
    /// - `expected_gateway_id`: Identifier of the User VM whose `GatewayReady` is expected.
    /// - `gateway_timeout`: Maximum duration to wait for the notification.
    ///
    /// # Returns
    ///
    /// On success, returns `Ok(())`. On failure or timeout, returns an error.
    ///
    pub async fn wait_for_gateway_ready(
        &self,
        expected_gateway_id: u32,
        gateway_timeout: Duration,
    ) -> Result<()> {
        let mut locked_inner = self.inner.lock().await;
        let inner: &mut LinuxDaemonInner = locked_inner.as_mut().ok_or_else(|| {
            let reason: &str = "inner state already taken";
            error!("wait_for_gateway_ready(): {reason}");
            anyhow::anyhow!("{reason}")
        })?;

        crate::sandbox::gateway_ready::wait_for_gateway_ready(
            &mut inner.control_plane_stream,
            &mut inner.pending_gateway_ready,
            expected_gateway_id,
            gateway_timeout,
        )
        .await
    }

    ///
    /// # Description
    ///
    /// Shuts down the Linux Daemon instance.
    ///
    /// # Notes
    ///
    /// - The method is idempotent - calling it multiple times is safe and has no effect after the
    ///   first successful shutdown.
    ///
    pub async fn shutdown(&self) {
        trace!("shutdown()");

        // Proceed with shutdown if we have the inner state.
        let Some(LinuxDaemonInner {
            mut control_plane_stream,
            mut child,
            pending_gateway_ready: _,
        }) = self.inner.lock().await.take()
        else {
            warn!("shutdown(): inner state already taken, skipping shutdown");
            return;
        };

        // Prepare shutdown message.
        let msg_bytes: [u8; NanvixdControlMessage::WIRE_SIZE] = {
            let msg: NanvixdControlMessage = NanvixdControlMessage::new(NanvixdCommand::Shutdown);
            let mut msg_bytes: [u8; NanvixdControlMessage::WIRE_SIZE] =
                [0u8; NanvixdControlMessage::WIRE_SIZE];
            msg.to_bytes(&mut msg_bytes);
            msg_bytes
        };

        // Send shutdown command to Linux Daemon.
        if let Err(error) = control_plane_stream.write_all(&msg_bytes).await {
            if error.kind() != ErrorKind::BrokenPipe {
                error!("shutdown(): failed to send shutdown command to linuxd (error={error:?})");
            }
        }

        // Wait for linuxd instance to finish.
        match timeout(SHUTDOWN_TIMEOUT, child.wait()).await {
            Ok(Ok(exit_status)) => {
                if !exit_status.success() {
                    warn!(
                        "shutdown(): linuxd returned with non-zero exit status (status={:?})",
                        exit_status.code()
                    );
                }
            },
            Ok(Err(error)) => {
                warn!("shutdown(): error waiting for linuxd (error={error:?})");
                Self::send_sigkill_to_child(&child);
            },
            Err(elapsed) => {
                warn!("shutdown(): timed-out waiting for linuxd (error={elapsed:?})");
                Self::send_sigkill_to_child(&child);
            },
        }
    }

    /// Reproduces the old buggy behavior that discards non-matching `GatewayReady` messages
    /// instead of buffering them. Used only by regression tests to prove the fix is necessary.
    #[cfg(test)]
    async fn wait_for_gateway_ready_no_buffer(
        &self,
        expected_gateway_id: u32,
        gateway_timeout: Duration,
    ) -> Result<()> {
        let mut locked_inner = self.inner.lock().await;
        let inner: &mut LinuxDaemonInner = locked_inner
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("inner state already taken"))?;

        crate::sandbox::gateway_ready::wait_for_gateway_ready_no_buffer(
            &mut inner.control_plane_stream,
            expected_gateway_id,
            gateway_timeout,
        )
        .await
    }

    /// Creates a `LinuxDaemon` backed by a dummy child process and the given socket stream. This
    /// allows unit tests to exercise `wait_for_gateway_ready` without spawning a real linuxd.
    #[cfg(test)]
    fn new_for_test(control_plane_stream: SocketStream) -> Self {
        // Spawn a trivial long-lived child so `LinuxDaemonInner` has a valid `Child`.
        let child: Child = Command::new("sleep")
            .arg("60")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn dummy child for test");
        Self {
            inner: Mutex::new(Some(LinuxDaemonInner {
                child,
                control_plane_stream,
                pending_gateway_ready: HashMap::new(),
            })),
        }
    }
}

impl PendingLinuxDaemon {
    ///
    /// # Description
    ///
    /// Completes Linux daemon startup by attaching the accepted control-plane stream.
    ///
    /// # Arguments
    ///
    /// - `control_plane_stream`: Accepted control-plane stream for the spawned Linux daemon.
    ///
    /// # Returns
    ///
    /// Returns the running Linux daemon handle.
    ///
    pub fn attach_control_plane(self, control_plane_stream: SocketStream) -> LinuxDaemon {
        LinuxDaemon {
            inner: Mutex::new(Some(LinuxDaemonInner {
                child: self.child,
                control_plane_stream,
                pending_gateway_ready: HashMap::new(),
            })),
        }
    }

    ///
    /// # Description
    ///
    /// Aborts a pending Linux daemon startup by forcefully terminating the child process.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    ///
    pub async fn abort(self) {
        LinuxDaemon::send_sigkill_to_child(&self.child);
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[path = "../gateway_ready_tests.rs"]
mod tests;
