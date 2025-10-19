// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::control_plane_api::{
    NanvixdCommand,
    NanvixdControlMessage,
};
use ::hwloc::HwLoc;
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
    sync::Mutex,
    task::JoinHandle,
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
    /// - `control_plane_sockaddr`: Control-plane socket address.
    /// - `user_vm_sockaddr`: User-VM socket address.
    /// - `hwloc`: Optional CPU affinity settings (ignored)
    /// - `binary_directory`: Directory containing toolchain binaries (not used).
    /// - `toolchain_binary_directory`: Directory containing toolchain binaries (not used).
    /// - `log_directory`: Directory to store logs (not used).
    /// - `control_plane_listener`: Control-plane socket listener.
    /// - `l2`: Whether to spawn the Linux Daemon in L2 mode (not supported).
    /// - `tmp_directory`: Temporary directory (not used).
    ///
    /// # Return Value
    ///
    /// On success, this function returns a future that, when resolve yields a handle to the spawned
    /// Linux Daemon instance. On failure, this function returns an error object instead.
    ///
    #[allow(clippy::too_many_arguments)]
    pub async fn spawn(
        control_plane_sockaddr: &str,
        user_vm_sockaddr: &str,
        hwloc: Option<HwLoc>,
        _binary_directory: &str,
        _toolchain_binary_directory: &str,
        _log_directory: &str,
        control_plane_listener: &mut SocketListener,
        l2: bool,
        _tmp_directory: String,
    ) -> Result<Self> {
        trace!(
            "spawn(): control_plane_sockaddr={control_plane_sockaddr}, \
             user_vm_sockaddr={user_vm_sockaddr}"
        );

        // Check if CPU affinity settings were provided.
        if let Some(hwloc) = hwloc {
            warn!("spawn(): single-process mode ignores hwloc affinity settings (hwloc={hwloc:?})");
        }

        // Check if L2 mode was requested.
        if l2 {
            let reason: &str = "single-process mode does not support L2 deployments";
            error!("spawn(): {reason}");
            anyhow::bail!("{reason}");
        }

        // Create a socket to listen for user VM connections.
        let user_vm_listener: SocketListener = UnboundSocket::new(SocketType::Unix)
            .bind(user_vm_sockaddr.to_string())
            .await
            .map_err(|e| {
                error!(
                    "spawn(): failed to bind linuxd user VM listener (address={user_vm_sockaddr}, \
                     error={e:?})"
                );
                anyhow::anyhow!("failed to bind linuxd user VM listener")
            })?;

        // Create a new Linux Daemon instance.
        let linuxd: EmbeddedLinuxd = EmbeddedLinuxd::init(
            control_plane_sockaddr,
            SocketType::UNIX_STR,
            user_vm_listener,
            l2,
        )
        .map_err(|e| {
            error!("spawn(): failed to initialize linuxd (error={e:?})");
            anyhow::anyhow!("failed to initialize linuxd")
        })?;

        // Spawn a task to run the Linux Daemon.
        let linuxd_task: JoinHandle<Result<()>> = ::tokio::spawn(async move {
            let result = linuxd.run().await;
            if let Err(ref err) = result {
                error!("spawn(): linuxd terminated with error (error={err:?})");
            }
            result.map_err(|e| anyhow::anyhow!("linuxd run failed: {e:?}"))
        });

        // Wait for the linuxd to connect to the control-plane socket.
        let control_plane_stream: SocketStream = match timeout(
            Duration::from_secs(config::syscomm::ACCEPT_TIMEOUT_SECS),
            control_plane_listener.accept(),
        )
        .await
        {
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
            match timeout(
                Duration::from_secs(::config::syscomm::SHUTDOWN_TIMEOUT_SECS),
                linuxd_task,
            )
            .await
            {
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
