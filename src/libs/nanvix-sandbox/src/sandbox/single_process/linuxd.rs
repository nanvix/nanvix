// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Linux Daemon management for single-process mode.
//!
//! This module provides functionality to spawn and manage Linux Daemon instances as async
//! tasks within the same process. This mode is primarily used for testing and development,
//! avoiding the overhead of process creation and simplifying debugging.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::{
        CONTROL_PLANE_ACCEPT_TIMEOUT,
        SHUTDOWN_TIMEOUT,
    },
    LinuxDaemonArgs,
};
use ::anyhow::Result;
use ::control_plane_api::{
    NanvixdCommand,
    NanvixdControlMessage,
};
use ::linuxd::LinuxDaemon as EmbeddedLinuxd;
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::std::collections::HashMap;
use ::syscomm::{
    SocketListener,
    SocketStream,
    SocketType,
    UnboundSocket,
    WriteAll,
};
use ::tokio::{
    runtime::Handle,
    sync::Mutex,
    task::{
        self,
        JoinHandle,
    },
    time::timeout,
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
    /// Underlying task.
    linuxd_task: JoinHandle<Result<()>>,
    /// Control-plane socket stream.
    control_plane_stream: SocketStream,
    /// Set of gateway IDs for which a `GatewayReady` notification has already been received but not
    /// yet claimed by the corresponding caller.
    pending_gateway_ready: HashMap<u32, usize>,
}

///
/// # Description
///
/// Handle to a running Linux Daemon instance.
///
pub struct LinuxDaemon {
    /// Interior mutable state.
    inner: Mutex<Option<LinuxDaemonInner>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    ///
    /// # Description
    ///
    /// Spawns a new Linux Daemon instance as a task in the current process.
    ///
    /// # Parameters
    ///
    /// - `args`: Linux Daemon arguments.
    /// - `control_plane_listener`: Control-plane socket listener.
    ///
    /// # Returns
    ///
    /// On success, this function returns a handle to the spawned Linux Daemon instance. On failure,
    /// this function returns an error object instead.
    ///
    pub async fn spawn<T: Sync + Send + Default + 'static>(
        args: &LinuxDaemonArgs<T>,
        control_plane_listener: &mut SocketListener,
    ) -> Result<Self> {
        trace!(
            "spawn(): control_plane_connect_socket_address={:?}, user_vm_sockaddr={:?}",
            args.control_plane_connect_socket_info(),
            args.system_vm_socket_info()
        );

        // Check if CPU affinity settings were provided.
        if let Some(hwloc) = args.hwloc() {
            warn!("spawn(): single-process mode ignores hwloc affinity settings (hwloc={hwloc:?})");
        }

        // Check if L2 mode was requested.
        if args.l2() {
            let reason: &str = "single-process mode does not support L2 deployments";
            error!("spawn(): {reason}");
            anyhow::bail!("{reason}");
        }

        // Create a socket to listen for user VM connections.
        let user_vm_listener: SocketListener = UnboundSocket::new(SocketType::Unix)
            .bind(&args.system_vm_socket_info().0)
            .await
            .map_err(|e| {
                error!(
                    "spawn(): failed to bind linuxd user VM listener (address={}, error={e:?})",
                    args.system_vm_socket_info().0
                );
                anyhow::anyhow!("failed to bind linuxd user VM listener")
            })?;

        // Create a new Linux Daemon instance.
        let syscall_table: ::std::sync::Arc<::linuxd::syscalls::SyscallTable<T>> =
            args.syscall_table().unwrap_or_else(|| {
                ::std::sync::Arc::new(::linuxd::syscalls::SyscallTable::new(T::default()))
            });

        let linuxd: EmbeddedLinuxd<T> = EmbeddedLinuxd::init(
            syscall_table,
            &args.control_plane_connect_socket_info().0,
            args.control_plane_connect_socket_info().1.to_str(),
            user_vm_listener,
            args.l2(),
        )
        .map_err(|e| {
            error!("spawn(): failed to initialize linuxd (error={e:?})");
            anyhow::anyhow!("failed to initialize linuxd")
        })?;

        // Spawn a task to run the Linux Daemon.
        let linuxd_task: JoinHandle<Result<()>> = task::spawn_blocking(move || {
            Handle::current().block_on(async move {
                let result = linuxd.run().await;
                if let Err(ref err) = result {
                    error!("spawn(): linuxd terminated with error (error={err:?})");
                }
                result.map_err(|e| anyhow::anyhow!("linuxd run failed: {e:?}"))
            })
        });

        // Wait for the linuxd to connect to the control-plane socket.
        let control_plane_stream: SocketStream =
            match timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, control_plane_listener.accept()).await {
                Ok(Ok(stream)) => stream,
                Ok(Err(error)) => {
                    linuxd_task.abort();
                    let reason: String =
                        format!("error connecting control-plane to linuxd (error={error:?})");
                    error!("spawn(): {reason}");
                    anyhow::bail!("{reason}");
                },
                Err(elapsed) => {
                    linuxd_task.abort();
                    let reason: String = format!(
                        "timed-out waiting for linuxd to connect the control-plane stream \
                         (elapsed={elapsed:?})"
                    );
                    error!("spawn(): {reason}");
                    anyhow::bail!("{reason}");
                },
            };

        debug!("spawn(): nanvixd received connection from linuxd control-plane socket");

        Ok(Self {
            inner: Mutex::new(Some(LinuxDaemonInner {
                linuxd_task,
                control_plane_stream,
                pending_gateway_ready: HashMap::new(),
            })),
        })
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
        gateway_timeout: ::tokio::time::Duration,
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
            linuxd_task,
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
            warn!("shutdown(): failed to send shutdown command to linuxd (error={error:?})");
        }

        // Wait for the Linux Daemon to finish.
        match timeout(SHUTDOWN_TIMEOUT, linuxd_task).await {
            Ok(join_result) => match join_result {
                Ok(Ok(())) => {},
                Ok(Err(error)) => {
                    warn!("shutdown(): linuxd terminated with error (error={error:?})");
                },
                Err(join_error) => {
                    warn!("shutdown(): failed to join linuxd task (error={join_error:?})");
                },
            },
            Err(elapsed) => {
                warn!("shutdown(): timed-out waiting for linuxd to shutdown (elapsed={elapsed:?})");
            },
        }
    }

    /// Reproduces the old buggy behavior that discards non-matching `GatewayReady` messages
    /// instead of buffering them. Used only by regression tests to prove the fix is necessary.
    #[cfg(test)]
    async fn wait_for_gateway_ready_no_buffer(
        &self,
        expected_gateway_id: u32,
        gateway_timeout: ::tokio::time::Duration,
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

    /// Creates a `LinuxDaemon` backed by a no-op task and the given socket stream. This allows
    /// unit tests to exercise `wait_for_gateway_ready` without spawning a real linuxd.
    #[cfg(test)]
    fn new_for_test(control_plane_stream: SocketStream) -> Self {
        // Spawn a trivial task that sleeps forever so `LinuxDaemonInner` has a valid handle.
        let linuxd_task: JoinHandle<Result<()>> = task::spawn(async {
            ::tokio::time::sleep(::tokio::time::Duration::from_secs(3600)).await;
            Ok(())
        });
        Self {
            inner: Mutex::new(Some(LinuxDaemonInner {
                linuxd_task,
                control_plane_stream,
                pending_gateway_ready: HashMap::new(),
            })),
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[path = "../gateway_ready_tests.rs"]
mod tests;
