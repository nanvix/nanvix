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
        CONTROL_PLANE_ACCEPT_TIMEOUT,
        SHUTDOWN_TIMEOUT,
    },
    netns::{
        NetnsHandle,
        NetnsInfo,
    },
    netns_exec::command_in_netns,
    LinuxDaemonArgs,
    SnapshotDirHandle,
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
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::std::{
    collections::HashMap,
    error::Error as StdError,
    fmt,
    fs,
    io::ErrorKind,
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
use ::tokio::{
    fs as tokio_fs,
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    process::{
        Child,
        ChildStderr,
        Command,
    },
    sync::Mutex,
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
// WaitForSocketError
//==================================================================================================

///
/// # Description
///
/// Error type returned when waiting for the Cloud Hypervisor API socket to show up.
///
#[derive(Debug)]
enum WaitForSocketError {
    /// Socket path exists but is not a socket.
    NotSocket {
        /// Path to the unexpected file.
        path: PathBuf,
    },
    /// Metadata lookup failed.
    Metadata {
        /// Path that triggered the error.
        path: PathBuf,
        /// Underlying I/O error.
        source: ::std::io::Error,
    },
    /// Socket never appeared within the allowed timeout.
    TimedOut {
        /// Path to the awaited socket.
        path: PathBuf,
    },
}

impl fmt::Display for WaitForSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSocket { path } => {
                write!(f, "file available, but not a socket (path={path:?})")
            },
            Self::Metadata { path, source } => {
                write!(f, "error checking file metadata (path={path:?}, error={source:?})")
            },
            Self::TimedOut { path } => {
                write!(f, "timed-out waiting for socket to be available (path={path:?})")
            },
        }
    }
}

impl StdError for WaitForSocketError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Metadata { source, .. } => Some(source),
            _ => None,
        }
    }
}

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
    /// RAII handle to the network namespace linuxd runs in (L2-mode only).
    netns_handle: Option<NetnsHandle>,
    /// RAII handle to the per-instance snapshot directory used in L2 mode.
    snapshot_dir_handle: Option<SnapshotDirHandle>,
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
    async fn wait_for_unix_socket<P: AsRef<Path>>(
        path: P,
        timeout_duration: Duration,
    ) -> ::std::result::Result<(), WaitForSocketError> {
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
                        error!(
                            "wait_for_unix_socket(): file available, but not a socket (path={:?})",
                            path
                        );
                        return Err(WaitForSocketError::NotSocket { path: path.clone() });
                    }
                },
                Err(e) if e.kind() == ErrorKind::NotFound => {},
                Err(e) => {
                    error!(
                        "wait_for_unix_socket(): error checking file metadata (path={:?}, \
                         error={:?})",
                        path, e
                    );
                    return Err(WaitForSocketError::Metadata {
                        path: path.clone(),
                        source: e,
                    });
                },
            }

            if Instant::now() >= deadline {
                error!(
                    "wait_for_unix_socket(): timed-out waiting for socket to be available \
                     (path={:?})",
                    path
                );
                return Err(WaitForSocketError::TimedOut { path });
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
    /// - `clh_stderr_log_path`: Destination file where CLH stderr is captured.
    ///
    /// # Returns
    ///
    /// On success, an empty tuple is returned. On failure, an error is returned instead.
    ///
    async fn resume_l2_vm(
        netns_info: &NetnsInfo,
        ch_remote_path: &str,
        clh_api_socket_path: &str,
        clh_stderr_log_path: &Path,
    ) -> Result<()> {
        // Timeout between the ch-remote resume operation and the API socket becoming available.
        const CLH_RESUME_TIMEOUT: Duration = Duration::from_secs(5);
        const CLH_SOCKET_MAX_ATTEMPTS: usize = 5;
        const CLH_SOCKET_BACKOFF_MS: u64 = 250;

        // Wait for CLH socket to be ready with retries to mask slow boots.
        let mut attempt: usize = 0;
        loop {
            match Self::wait_for_unix_socket(clh_api_socket_path, CLH_RESUME_TIMEOUT).await {
                Ok(()) => break,
                Err(WaitForSocketError::TimedOut { ref path }) => {
                    attempt += 1;
                    if attempt > CLH_SOCKET_MAX_ATTEMPTS {
                        let reason: String = format!(
                            "timed-out waiting for socket to be available (path={path:?}, \
                             attempts={attempt}, stderr_log={})",
                            clh_stderr_log_path.display()
                        );
                        error!("resume_l2_vm(): {reason}");
                        anyhow::bail!(reason);
                    }

                    warn!(
                        "resume_l2_vm(): attempt {attempt} timed out while waiting for socket \
                         (path={path:?}), retrying..."
                    );
                    let backoff_ms: u64 = CLH_SOCKET_BACKOFF_MS * attempt as u64;
                    sleep(Duration::from_millis(backoff_ms)).await;
                },
                Err(e) => {
                    let reason: String = format!(
                        "error waiting for socket readiness (error={e}, stderr_log={})",
                        clh_stderr_log_path.display()
                    );
                    error!("resume_l2_vm(): {reason}");
                    anyhow::bail!(reason);
                },
            }
        }

        // Resume the L2 VM inside the network namespace.
        let ch_remote_args: Vec<String> = vec![
            ch_remote_path.to_string(),
            args::Args::OPT_CLH_API_SOCKET.to_string(),
            clh_api_socket_path.to_string(),
            args::Args::OPT_CH_REMOTE_RESUME.to_string(),
        ];
        debug!("resume_l2_vm(): executing ch-remote with args: {}", ch_remote_args.join(" "));
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
    /// Spawns a background task that persists the stderr stream emitted by cloud-hypervisor.
    ///
    /// # Parameters
    ///
    /// - `stderr`: Pipe containing the process stderr stream.
    /// - `destination`: Path to the log file that will receive the captured output.
    ///
    fn spawn_stderr_capture_task(stderr: ChildStderr, destination: PathBuf) {
        ::tokio::spawn(async move {
            if let Err(error) = Self::persist_child_stderr(stderr, destination.clone()).await {
                warn!(
                    "spawn_stderr_capture_task(): failed to persist cloud-hypervisor stderr \
                     (path={:?}, error={error:?})",
                    destination
                );
            }
        });
    }

    ///
    /// # Description
    ///
    /// Persists stderr output emitted by cloud-hypervisor into a log file for later inspection.
    ///
    /// # Parameters
    ///
    /// - `stderr`: Pipe containing the stderr stream.
    /// - `destination`: Log file path.
    ///
    /// # Returns
    ///
    /// Returns success when all data is written. Errors indicate an I/O failure when reading from
    /// the process or writing to disk.
    ///
    async fn persist_child_stderr(stderr: ChildStderr, destination: PathBuf) -> Result<()> {
        if let Some(parent) = destination.parent() {
            tokio_fs::create_dir_all(parent).await?;
        }

        let mut reader: ChildStderr = stderr;
        let mut file: tokio_fs::File = tokio_fs::File::create(&destination).await?;
        let mut buffer: [u8; 4096] = [0; 4096];

        loop {
            let bytes_read: usize = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            file.write_all(&buffer[..bytes_read]).await?;
        }

        file.flush().await?;
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
        snapshot_dir_handle: Option<SnapshotDirHandle>,
    ) -> Result<Self> {
        debug!(
            "spawn(): spawning linux daemon (control-plane={:?}, user-vm={:?}, l2={})",
            args.control_plane_connect_socket_info(),
            args.system_vm_socket_info(),
            args.l2()
        );

        let clh_api_socket_path: String = get_clh_api_socket_path(args.tmp_directory());
        let mut linuxd_args: Vec<String> = if args.l2() {
            let snapshot_dir_handle: &SnapshotDirHandle =
                snapshot_dir_handle.as_ref().ok_or_else(|| {
                    let reason: &str = "snapshot directory handle not provided for L2 deployment";
                    error!("spawn(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
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
                format!("source_url=file://{}", snapshot_dir_handle.path().to_string_lossy(),),
            ]
        } else {
            vec![
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
            ]
        };
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
        let child: Child = if let Some(netns_handle) = &netns_handle {
            // In L2 deployments, we spawn linuxd inside a network namespace.
            debug_assert!(args.l2());

            let clh_stderr_log_path: PathBuf =
                PathBuf::from(format!("{}/cloud-hypervisor-stderr.log", args.log_directory()));

            let mut cmd: Command =
                command_in_netns(&netns_handle.netns_info()?, &linuxd_args[0], &linuxd_args[1..]);
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::piped());

            let mut child: Child = cmd.spawn().map_err(|e| {
                let reason: String =
                    format!("error spawning linuxd process in netns (error={e:?})");
                error!("spawn(): {reason}");
                anyhow::anyhow!(reason)
            })?;

            if let Some(stderr) = child.stderr.take() {
                Self::spawn_stderr_capture_task(stderr, clh_stderr_log_path.clone());
            } else {
                warn!(
                    "spawn(): failed to capture cloud-hypervisor stderr (log_path={})",
                    clh_stderr_log_path.display()
                );
            }

            let ch_remote_path: String =
                format!("{}/ch-remote", get_clh_bin_dir(args.toolchain_binary_directory())?);
            if let Err(e) = Self::resume_l2_vm(
                &netns_handle.netns_info()?,
                &ch_remote_path,
                &clh_api_socket_path,
                &clh_stderr_log_path,
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
            inner: Mutex::new(Some(LinuxDaemonInner {
                child,
                control_plane_stream,
                pending_gateway_ready: HashMap::new(),
            })),
            netns_handle,
            snapshot_dir_handle,
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

    ///
    /// # Description
    ///
    /// Returns the path to the per-instance snapshot directory, if any.
    ///
    /// # Returns
    ///
    /// The per-instance snapshot directory path in L2 mode, or `None` otherwise.
    ///
    pub fn snapshot_dir_path(&self) -> Option<&Path> {
        self.snapshot_dir_handle
            .as_ref()
            .map(SnapshotDirHandle::path)
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
            netns_handle: None,
            snapshot_dir_handle: None,
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[path = "../gateway_ready_tests.rs"]
mod tests;
