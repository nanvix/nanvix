// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Linux Daemon management for multi-process mode.
//!
//! This module provides functionality to spawn and manage Linux Daemon instances as separate
//! processes. It handles process lifecycle, control-plane communication, and supports both
//! native execution and L2 VM deployment using cloud-hypervisor.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::{
        get_clh_api_socket_path,
        get_clh_bin_dir,
        get_clh_snapshot_path,
        CONTROL_PLANE_ACCEPT_TIMEOUT,
        SHUTDOWN_TIMEOUT,
    },
    netns::{
        NetnsHandle,
        NetnsInfo,
    },
    netns_exec::{
        command_in_netns,
        spawn_in_netns,
    },
    LinuxDaemonArgs,
};
use ::anyhow::Result;
use ::control_plane_api::{
    NanvixdCommand,
    NanvixdControlMessage,
};
use ::linuxd::{
    args,
    config::restore_gate_sockaddr_builder,
};
use ::std::{
    fs,
    io::ErrorKind,
    mem,
    os::unix::fs::FileTypeExt,
    path::{
        Path,
        PathBuf,
    },
    process::{
        ExitStatus,
        Stdio,
    },
};
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
    process::{
        Child,
        Command,
    },
    time::{
        sleep,
        timeout,
        Duration,
        Instant,
    },
};

/// Single-byte that we send to unlock a linuxd instance restored from a snapshot. Anything that
/// triggers a readable event in the receiving socket should work.
const RESTORE_GATE_BYTES: [u8; 1] = [0];

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Handle to a running Linux Daemon instance spawned as a separate process.
///
pub struct LinuxDaemon {
    /// Child process handle.
    child: Child,
    /// Control-plane socket stream.
    control_plane_stream: SocketStream,
    /// RAII handle to the network namespace linuxd runs in (L2-mode only).
    netns_handle: Option<NetnsHandle>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    ///
    /// # Description
    ///
    /// Waits until a unix socket path appears, or times out.
    ///
    /// This only checks filesystem-level existence and that the node is a socket. It does *not*
    /// actually connect to it. It can be used to poll for UNIX sockets to be ready even if they
    /// are in a different network namespace (so we cannot `connect` to them).
    ///
    /// # Parameters
    ///
    /// - `path`: The path to the unix socket file.
    /// - `timeout_duration`: The maximum duration to wait for the socket to appear.
    ///
    /// # Returns
    ///
    /// On success, returns an empty tuple. On failure, returns an error.
    ///
    // FIXME(#1171): we will be able to get rid of this method once we have a programmatic way to
    // spawn a task inside a namespace without spawning a different process.
    async fn wait_for_unix_socket<P: AsRef<Path>>(
        path: P,
        timeout_duration: Duration,
    ) -> Result<()> {
        let path: PathBuf = path.as_ref().to_path_buf();
        let deadline: Instant = Instant::now() + timeout_duration;
        const SLEEP_DURATION: Duration = Duration::from_millis(1);

        loop {
            match fs::symlink_metadata(&path) {
                Ok(meta) => {
                    // Check file is a socket.
                    if meta.file_type().is_socket() {
                        return Ok(());
                    } else {
                        // Exists but is not a socket, raise error.
                        let reason: String =
                            format!("file available, but not a socket (path={path:?})");
                        error!("wait_for_unix_socket(): {reason}");
                        anyhow::bail!(reason);
                    }
                },
                Err(e) if e.kind() == ErrorKind::NotFound => {},
                Err(e) => {
                    let reason: String =
                        format!("error checking file metadata (path={path:?}, error={e:?})");
                    error!("wait_for_unix_socket(): {reason}");
                    anyhow::bail!(reason);
                },
            }

            if Instant::now() >= deadline {
                let reason: String =
                    format!("timed-out waiting for socket to be available (path={path:?})");
                error!("wait_for_unix_socket(): {reason}");
                anyhow::bail!(reason);
            }

            sleep(SLEEP_DURATION).await;
        }
    }

    ///
    /// # Description
    ///
    /// Helper method to resume linuxd from a snapshot.
    ///
    /// We need to do two steps after we restore linuxd's state from a snapshot (in an L2 VM).
    /// First we need to actually resume the VM's execution using cloud-hypervisor's API socket.
    /// Then we need to "unlock" linuxd from a pre-snapshot gate that we use to control exactly
    /// when the VM is snapshotted. Linuxd in an L2 VM executes in a separate network namespace, so
    /// we need to keep that in mind during restore.
    ///
    /// # Parameters
    ///
    /// - `netns_info`: Information about the L2 VM's network namespace.
    /// - `ch_remote_path`: Path to the ch-remote binary.
    /// - `clh_api_socket_path`: Path to the cloud-hypervisor API socket.
    ///
    /// # Returns
    ///
    /// On success, an empty tuple is returned. On failure, an error is returned instead.
    ///
    async fn resume_l2_vm(
        netns_info: &NetnsInfo,
        ch_remote_path: &str,
        clh_api_socket_path: &str,
    ) -> Result<()> {
        // Timeout between the ch-remote resume operation and the API socket becoming available.
        const CLH_RESUME_TIMEOUT: Duration = Duration::from_millis(100);

        // Wait for CLH socket to be ready.
        Self::wait_for_unix_socket(clh_api_socket_path, CLH_RESUME_TIMEOUT).await?;

        // Resume the L2 VM inside the network namespace.
        let ch_remote_args: Vec<String> = vec![
            ch_remote_path.to_string(),
            args::Args::OPT_CLH_API_SOCKET.to_string(),
            clh_api_socket_path.to_string(),
            args::Args::OPT_CH_REMOTE_RESUME.to_string(),
        ];
        trace!("ch-remote args: {ch_remote_args:?}");
        let status: ExitStatus =
            command_in_netns(netns_info, &ch_remote_args[0], &ch_remote_args[1..])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await
                .map_err(|e| {
                    let reason: String = format!(
                        "error spawning ch remote process (args={ch_remote_args:?}, error={e:?})"
                    );
                    error!("{reason}");
                    anyhow::anyhow!(reason)
                })?;
        if !status.success() {
            let reason: String = format!(
                "error running ch remote process (args={ch_remote_args:?}, status={status:?})"
            );
            error!("{reason}");
            anyhow::bail!(reason);
        }

        // After receiving the HTTP reply, unlock the post-snapshot gate by sending a single byte.
        let unbound_socket: UnboundSocket = UnboundSocket::new(SocketType::Tcp);
        let mut stream: SocketStream = unbound_socket
            .connect(&restore_gate_sockaddr_builder(Some(netns_info.veth_ns_ip())))
            .await?;
        if let Err(e) = stream.write_all(&RESTORE_GATE_BYTES).await {
            error!("failed to write restore gate bytes (error={e:?})");
            return Err(e.into());
        }

        Ok(())
    }

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
    /// - `control_plane_listener`: Control-plane socket listener.
    /// - `netns_handle`: Optional handle to a network namespace (L2-mode only).
    ///
    /// # Returns
    ///
    /// On success, this function returns a handle to the spawned Linux Daemon instance. On failure,
    /// this function returns an error object instead.
    ///
    pub async fn spawn<T: Sync + Send + 'static>(
        args: &LinuxDaemonArgs<T>,
        control_plane_listener: &mut SocketListener,
        netns_handle: Option<NetnsHandle>,
    ) -> Result<Self> {
        debug!(
            "spawning linux daemon (control-plane={:?}, user-vm={:?}, l2={})",
            args.control_plane_socket_info(),
            args.system_vm_socket_info(),
            args.l2()
        );

        let clh_api_socket_path: String = get_clh_api_socket_path(args.tmp_directory());
        let mut linuxd_args: Vec<String> = if args.l2() {
            match ::std::fs::remove_file(&clh_api_socket_path) {
                Ok(()) => {},
                Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => {},
                Err(e) => {
                    let reason: String = format!("error removing clh socket file (error={e:?})");
                    error!("spawn(): {reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            };

            vec![
                format!("{}/cloud-hypervisor", get_clh_bin_dir(args.toolchain_binary_directory())?),
                args::Args::OPT_CLH_API_SOCKET.to_string(),
                clh_api_socket_path.clone(),
                // FIXME(#1156): re-enable --seccomp true (default) when we cut a new Nanvix
                // release.
                args::Args::OPT_CLH_SECCOMP.to_string(),
                "false".to_string(),
                args::Args::OPT_CLH_RESTORE.to_string(),
                format!("source_url=file://{}", get_clh_snapshot_path(args.l2_snapshot_path())?),
            ]
        } else {
            vec![
                args.linuxd_binary_path().to_string(),
                args::Args::OPT_LOGFILE.to_string(),
                args::Args::OPT_LOGDIR.to_string(),
                args.log_directory().to_string(),
                args::Args::OPT_CONTROL_PLANE_SOCKADDR.to_string(),
                args.control_plane_socket_info().0.clone(),
                args::Args::OPT_CONTROL_PLANE_SOCKET_TYPE.to_string(),
                args.control_plane_socket_info().1.to_str().to_string(),
                args::Args::OPT_USER_VM_BIND_SOCKADDR.to_string(),
                args.system_vm_socket_info().0.clone(),
                args::Args::OPT_USER_VM_BIND_SOCKET_TYPE.to_string(),
                args.system_vm_socket_info().1.to_str().to_string(),
            ]
        };
        trace!("linuxd args: {:?}", linuxd_args);
        if let Some(hwloc) = args.hwloc() {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                hwloc.get_linuxd_core_str(),
            ];
            linuxd_args.splice(0..0, taskset);
        }

        // Inherit stdout/stderr so that errors when spawning the command are surfaced to nanvixd.
        let child: Child = if let Some(netns_handle) = &netns_handle {
            // In L2 deployments, we spawn linuxd inside a network namespace.
            debug_assert!(args.l2());

            let child: Child =
                spawn_in_netns(&netns_handle.netns_info()?, &linuxd_args[0], &linuxd_args[1..])
                    .await
                    .map_err(|e| {
                        let reason: String =
                            format!("error spawning linuxd process in netns (error={e:?})");
                        error!("spawn(): {reason}");
                        anyhow::anyhow!(reason)
                    })?;

            let ch_remote_path: String =
                format!("{}/ch-remote", get_clh_bin_dir(args.toolchain_binary_directory())?);
            if let Err(e) = Self::resume_l2_vm(
                &netns_handle.netns_info()?,
                &ch_remote_path,
                &clh_api_socket_path,
            )
            .await
            {
                let reason: String = format!("error resuming L2 VM (error={e:?})");
                error!("spawn(): {reason}");

                // Use a SIGKILL because the process is already faulty.
                Self::send_sigkill_to_child(&child);

                return Err(anyhow::anyhow!(reason));
            }

            child
        } else {
            Command::new(&linuxd_args[0])
                .args(&linuxd_args[1..])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| {
                    let reason: String = format!("error spawning linuxd process (error={e:?})");
                    error!("spawn(): {reason}");
                    anyhow::anyhow!(reason)
                })?
        };

        // After linuxd has started, accept the incoming connection and return the stream for
        // further use.
        let control_plane_stream: SocketStream =
            match timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, control_plane_listener.accept()).await {
                Ok(Ok(stream)) => stream,
                Ok(Err(e)) => {
                    // If linuxd has not accepted the control-plane connection, it means that
                    // something went wrong during start-up. We kill the process ignoring errors,
                    // and return an error.
                    let reason: String =
                        format!("error connecting control-plane to linuxd (error={e:?})");
                    error!("spawn(): {reason}");

                    // Use a SIGKILL because the process is already faulty.
                    Self::send_sigkill_to_child(&child);

                    anyhow::bail!(reason)
                },
                Err(e) => {
                    let reason: String = format!(
                        "timed-out waiting for linuxd to connect to control-plane (error={e:?})"
                    );
                    error!("spawn(): {reason}");

                    // Use a SIGKILL because the process is already faulty.
                    Self::send_sigkill_to_child(&child);

                    anyhow::bail!(reason)
                },
            };
        debug!("nanvixd received connection from linuxd's control-plane socket");

        Ok(Self {
            child,
            control_plane_stream,
            netns_handle,
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
            if error.kind() != ErrorKind::BrokenPipe {
                error!("shutdown(): failed to send shutdown command to linuxd (error={error:?})");
            }
        }

        // Wait for linuxd instance to finish.
        match timeout(SHUTDOWN_TIMEOUT, self.child.wait()).await {
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
                Self::send_sigkill_to_child(&self.child);
            },
            Err(elapsed) => {
                warn!("shutdown(): timed-out waiting for linuxd (error={elapsed:?})");
                Self::send_sigkill_to_child(&self.child);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Share ownership of the network namespace by passing a copy. This method is used to share a
    /// network namespace between linuxd and the user VMs mapped to it.
    ///
    /// # Returns
    ///
    /// A cloned handle to the network namespace if available, or `None` otherwise.
    ///
    pub fn netns_handle(&self) -> Option<NetnsHandle> {
        self.netns_handle.clone()
    }
}
