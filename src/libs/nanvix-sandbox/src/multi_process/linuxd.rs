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
    LinuxDaemonArgs,
};
use ::anyhow::Result;
use ::control_plane_api::{
    NanvixdCommand,
    NanvixdControlMessage,
};
use ::linuxd::{
    args,
    config::{
        restore_gate_sockaddr_builder,
        CONTROL_PLANE_CONNECT_TIMEOUT,
    },
};
use ::std::{
    io::ErrorKind,
    mem,
    process::Stdio,
};
use ::syscomm::{
    ReadExact,
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
    time::timeout,
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
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    ///
    /// # Description
    ///
    /// Helper method to resume linuxd from a snapshot.
    ///
    /// We need to do two steps after we restore linuxd's state from a snapshot (in an L2 VM).
    /// First we need to actually resume the VM's execution using cloud-hypervisor's API socket.
    /// Then we need to "unlock" linuxd from a pre-snapshot gate that we use to control exactly
    /// when the VM is snapshotted.
    ///
    /// # Parameters
    ///
    /// - `clh_api_socket_path`: Path to the cloud-hypervisor API socket.
    ///
    /// # Returns
    ///
    /// On success, an empty tuple is returned. On failure, an error is returned instead.
    ///
    async fn resume_l2_vm(clh_api_socket_path: &str) -> Result<()> {
        let resume_req: &str = concat!(
            "PUT /api/v1/vm.resume HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Accept: */*\r\n",
            "Content-Length: 0\r\n",
            "\r\n",
        );

        let unbound_clh_api_socket: UnboundSocket = UnboundSocket::new(SocketType::Unix);
        let now: tokio::time::Instant = tokio::time::Instant::now();
        let mut clh_api_socket: SocketStream = loop {
            match unbound_clh_api_socket
                .clone()
                .connect(clh_api_socket_path)
                .await
            {
                Ok(stream) => {
                    debug!(
                        "clh API socket appeared after {:?} (path={:?})",
                        now.elapsed(),
                        &clh_api_socket_path
                    );
                    break stream;
                },
                Err(e) => {
                    // Tolerate transient connection errors.
                    if matches!(
                        e.kind(),
                        ErrorKind::NotFound | ErrorKind::ConnectionRefused | ErrorKind::WouldBlock
                    ) {
                        if now.elapsed().as_secs() > CONTROL_PLANE_CONNECT_TIMEOUT.as_secs() {
                            let reason: String = format!(
                                "error connecting to CLH API socket (addr={}, error=timed-out)",
                                clh_api_socket_path
                            );
                            error!("{reason}");
                            return Err(anyhow::anyhow!(reason));
                        } else {
                            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                            continue;
                        }
                    }

                    // Bail on fatal errors.
                    let reason: String = format!(
                        "error connecting to CLH API socket (addr={}, error={e:?})",
                        clh_api_socket_path
                    );
                    error!("{reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            }
        };

        // Write HTTP request.
        // TODO: this request/response flow takes a considerable portion of the restore process
        // (almost half). We should investigate why is this the case, and whether it is a
        // fundamental limitation.
        if let Err(e) = clh_api_socket.write_all(resume_req.as_bytes()).await {
            error!("failed to write resume request to CLH API socket (error={e:?})");
            return Err(e.into());
        }

        // Wait for at least one byte of the reply, otherwise cloud-hypervisor hangs.
        let mut buf: [u8; 1] = [0u8; 1];
        if let Err(e) = clh_api_socket.read_exact(&mut buf).await {
            error!("failed to read resume response from CLH API socket (error={e:?})");
            return Err(e.into());
        }

        // After receiving the HTTP reply, unlock the post-snapshot gate by sending a single byte.
        let unbound_socket: UnboundSocket = UnboundSocket::new(SocketType::Tcp);
        let mut stream: SocketStream = unbound_socket
            .connect(&restore_gate_sockaddr_builder())
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
        debug!(
            "spawning linux daemon (control-plane={:?}, user-vm={:?}, l2={})",
            args.control_plane_socket_info(),
            args.system_vm_socket_info(),
            args.l2()
        );

        let clh_api_socket_path: String = get_clh_api_socket_path(args.tmp_directory());
        let mut linuxd_args: Vec<String> = if args.l2() {
            match std::fs::remove_file(&clh_api_socket_path) {
                Ok(()) => {},
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
                Err(e) => {
                    let reason: String = format!("error removing clh socket file (error={e:?})");
                    error!("{reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            };

            vec![
                format!("{}/cloud-hypervisor", get_clh_bin_dir(args.toolchain_binary_directory())?),
                args::Args::OPT_CLH_API_SOCKET.to_string(),
                clh_api_socket_path.clone(),
                args::Args::OPT_CLH_RESTORE.to_string(),
                format!("source_url=file://{}", get_clh_snapshot_path()),
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
        let child: Child = Command::new(&linuxd_args[0])
            .args(&linuxd_args[1..])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        if args.l2() {
            if let Err(e) = Self::resume_l2_vm(&clh_api_socket_path).await {
                let reason: String = format!("error resuming L2 VM (error={e:?})");
                error!("{reason}");

                // Use a SIGKILL because the process is already faulty.
                Self::send_sigkill_to_child(&child);

                return Err(anyhow::anyhow!("{reason}"));
            }
        }

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
                    error!("{reason}");

                    // Use a SIGKILL because the process is already faulty.
                    Self::send_sigkill_to_child(&child);

                    anyhow::bail!("{reason}")
                },
                Err(e) => {
                    let reason: String = format!(
                        "timed-out waiting for linuxd to connect to control-plane (error={e:?})"
                    );
                    error!("{reason}");

                    // Use a SIGKILL because the process is already faulty.
                    Self::send_sigkill_to_child(&child);

                    anyhow::bail!("{reason}")
                },
            };
        debug!("nanvixd received connection from linuxd's control-plane socket");

        Ok(Self {
            child,
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
}
