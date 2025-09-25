// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config::{
    get_clh_api_socket_path,
    get_clh_bin_dir,
    get_clh_snapshot_path,
};
use ::anyhow::Result;
use ::control_plane_api;
use ::hwloc::HwLoc;
use ::linuxd::{
    args,
    config::restore_gate_sockaddr_builder,
};
use ::mio::Poll;
use ::std::{
    process::Stdio,
    time::Duration,
};
use ::syscomm::{
    BlockingSocketStream,
    SocketListener,
    SocketStream,
    SocketType,
};
use ::syslog::{
    debug,
    error,
};
use ::tokio::process::{
    Child,
    Command,
};

/// Single-byte that we send to unlock a linuxd instance restored from a snapshot. Anything that
/// triggers a readable event in the receiving socket should work.
const RESTORE_GATE_BYTES: [u8; 1] = [0];

//==================================================================================================
// Structures
//==================================================================================================

pub struct LinuxDaemon {
    child: Child,
    control_plane_stream: SocketStream,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    /// Helper method to resume linuxd from a snapshot.
    ///
    /// We need to do two steps after we restore linuxd's state from a snapshot (in an L2 VM).
    /// First we need to actually resume the VM's execution using cloud-hypervisor's API socket.
    /// Then we need to "unlock" linuxd from a pre-snapshot gate that we use to control exactly
    /// when the VM is snapshotted.
    fn resume_l2_vm(clh_api_socket_path: String) -> Result<()> {
        let resume_req: &str = concat!(
            "PUT /api/v1/vm.resume HTTP/1.1\r\n",
            "Host: localhost\r\n",
            "Accept: */*\r\n",
            "Content-Length: 0\r\n",
            "\r\n",
        );

        let mut clh_api_socket: BlockingSocketStream = match SocketStream::connect_timeout(
            SocketType::Unix,
            clh_api_socket_path.clone(),
            Duration::from_secs(config::syscomm::CONNECT_TIMEOUT_SECS),
        ) {
            Ok(stream) => stream.set_blocking()?,
            Err(e) => {
                let reason: String = format!("error connecting to CLH API socket (error={e:?})");
                error!("{reason}");
                return Err(anyhow::anyhow!(reason));
            },
        };

        // Write HTTP request.
        // TODO: this request/response flow takes a considerable portion of the restore process
        // (almost half). We should investigate why is this the case, and whether it is a
        // fundamental limitation.
        if let Err(e) = clh_api_socket.write_all(resume_req.as_bytes()) {
            error!("failed to write resume request to CLH API socket (error={e:?})");
            return Err(e.into());
        }

        // Wait for at least one byte of the reply, otherwise cloud-hypervisor hangs.
        let mut buf: [u8; 1] = [0u8; 1];
        if let Err(e) = clh_api_socket.read(&mut buf) {
            error!("failed to read resume response from CLH API socket (error={e:?})");
            return Err(e.into());
        }

        // After receiving the HTTP reply, unlock the post-snapshot gate by sending a single byte.
        let mut stream: BlockingSocketStream =
            SocketStream::connect(SocketType::Tcp, restore_gate_sockaddr_builder())?
                .set_blocking()?;
        Ok(stream.write_all(&RESTORE_GATE_BYTES)?)
    }

    fn send_sigkill_to_child(child: Child) {
        if let Some(pid) = child.id() {
            debug!("killing linuxd instance (pid={pid:?})");
            let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        control_plane_sockaddr: &str,
        user_vm_sockaddr: &str,
        hwloc: Option<HwLoc>,
        binary_directory: &str,
        toolchain_binary_directory: &str,
        control_plane_listener: &mut SocketListener,
        control_plane_poll: &mut Poll,
        l2: bool,
        tmp_directory: String,
    ) -> Result<Self> {
        debug!(
            "spawning linux daemon (control-plane={control_plane_sockaddr}, \
             user-vm={user_vm_sockaddr}, l2={l2})"
        );

        let clh_api_socket_path: String = get_clh_api_socket_path(&tmp_directory);
        let mut linuxd_args: Vec<String> = if l2 {
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
                format!("{}/cloud-hypervisor", get_clh_bin_dir(toolchain_binary_directory)?),
                args::Args::OPT_CLH_API_SOCKET.to_string(),
                clh_api_socket_path.clone(),
                args::Args::OPT_CLH_RESTORE.to_string(),
                format!("source_url=file://{}", get_clh_snapshot_path()),
            ]
        } else {
            vec![
                format!("{}/linuxd.elf", binary_directory),
                args::Args::OPT_LOGFILE.to_string(),
                args::Args::OPT_CONTROL_PLANE_SOCKADDR.to_string(),
                control_plane_sockaddr.to_string(),
                args::Args::OPT_USER_VM_BIND_SOCKADDR.to_string(),
                user_vm_sockaddr.to_string(),
            ]
        };
        if let Some(hwloc) = hwloc {
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

        if l2 {
            if let Err(e) = Self::resume_l2_vm(clh_api_socket_path) {
                let reason: String = format!("error resuming L2 VM (error={e:?})");
                error!("{reason}");

                // Use a SIGKILL because the process is already faulty.
                Self::send_sigkill_to_child(child);

                return Err(anyhow::anyhow!("{reason}"));
            }
        }

        // After linuxd has started, accept the incoming connection and return the stream for
        // further use.
        let control_plane_stream: SocketStream = match control_plane_listener.accept_timeout(
            control_plane_poll,
            Duration::from_secs(config::syscomm::ACCEPT_TIMEOUT_SECS),
        ) {
            Ok(stream) => stream,
            Err(e) => {
                // If linuxd has not accepted the control-plane connection, it means that
                // something went wrong during start-up. We kill the process ignoring errors,
                // and return an error.
                let reason: String =
                    format!("error connecting control-plane to linuxd (error={e:?})");
                error!("{reason}");

                // Use a SIGKILL because the process is already faulty.
                Self::send_sigkill_to_child(child);

                return Err(anyhow::anyhow!("{reason}"));
            },
        };
        debug!("nanvixd received connection from linuxd's control-plane socket");

        Ok(Self {
            child,
            control_plane_stream,
        })
    }

    /// Send a shutdown message to linuxd so that it can clean-up its internal resources.
    pub async fn shutdown(&mut self) -> Result<()> {
        match control_plane_api::send_command(
            &mut self.control_plane_stream,
            control_plane_api::Command::Shutdown,
        ) {
            Ok(()) => {},
            Err(e) => {
                error!("failed to send shutdown command to linuxd (error={e:?})");
            },
        };

        // Wait for linuxd instance to finish.
        match self.child.wait().await {
            Ok(exit_status) => {
                if !exit_status.success() {
                    let reason: String = format!(
                        "linuxd returned with non-zero exit status: {:?}",
                        exit_status.code()
                    );
                    error!("{reason}");

                    Err(anyhow::anyhow!(reason))
                } else {
                    Ok(())
                }
            },
            Err(e) => {
                let reason: String = format!("error waiting for linuxd: {e:?}");
                error!("{reason}");

                Err(anyhow::anyhow!(reason))
            },
        }
    }
}
