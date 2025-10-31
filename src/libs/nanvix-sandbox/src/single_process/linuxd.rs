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
use ::std::mem;
use ::syscomm::{
    SocketListener,
    SocketStream,
    SocketType,
    UnboundSocket,
    WriteAll,
};
use ::syslog::{
    debug,
    error,
    trace,
    warn,
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
/// Handle to a running Linux Daemon instance.
///
pub struct LinuxDaemon {
    /// Underlying task.
    linuxd_task: Mutex<Option<JoinHandle<Result<()>>>>,
    /// Control-plane socket stream.
    control_plane_stream: SocketStream,
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
    pub async fn spawn(
        args: &LinuxDaemonArgs,
        control_plane_listener: &mut SocketListener,
    ) -> Result<Self> {
        trace!(
            "spawn(): control_plane_socket_address={:?}, user_vm_sockaddr={:?}",
            args.control_plane_socket_info(),
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
        let linuxd: EmbeddedLinuxd = EmbeddedLinuxd::init(
            args.syscall_table().unwrap_or_default(),
            &args.control_plane_socket_info().0,
            args.control_plane_socket_info().1.to_str(),
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
            linuxd_task: Mutex::new(Some(linuxd_task)),
            control_plane_stream,
        })
    }

    ///
    /// # Description
    ///
    /// Shuts down the Linux Daemon instance.
    ///
    pub async fn shutdown(&mut self) {
        trace!("shutdown()");

        // Prepare shutdown message.
        let msg_bytes: [u8; mem::size_of::<NanvixdControlMessage>()] = {
            let msg: NanvixdControlMessage = NanvixdControlMessage::new(NanvixdCommand::Shutdown);
            let mut msg_bytes: [u8; mem::size_of::<NanvixdControlMessage>()] =
                [0u8; ::std::mem::size_of::<NanvixdControlMessage>()];
            msg.to_bytes(&mut msg_bytes);
            msg_bytes
        };

        // Send shutdown command to Linux Daemon.
        if let Err(error) = self.control_plane_stream.write_all(&msg_bytes).await {
            warn!("shutdown(): failed to send shutdown command to linuxd (error={error:?})");
        }

        // Wait for the Linux Daemon to finish.
        if let Some(linuxd_task) = self.linuxd_task.lock().await.take() {
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
                    warn!(
                        "shutdown(): timed-out waiting for linuxd to shutdown \
                         (elapsed={elapsed:?})"
                    );
                },
            }
        }
    }
}
