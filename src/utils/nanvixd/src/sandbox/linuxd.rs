// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::hwloc::HwLoc;
use ::linuxd::control_plane;
use ::mio::Poll;
use ::std::{
    process::Stdio,
    time::Duration,
};
use ::syscomm::{
    SocketListener,
    SocketStream,
};
use ::tokio::process::{
    Child,
    Command,
};

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
    pub fn spawn(
        control_plane_sockaddr: &str,
        user_vm_sockaddr: &str,
        gateway_sockaddr: &str,
        hwloc: Option<HwLoc>,
        binary_directory: &str,
        control_plane_listener: &mut SocketListener,
        control_plane_poll: &mut Poll,
    ) -> Result<Self> {
        debug!(
            "spawning linux daemon (control-plane={control_plane_sockaddr}, \
             user-vm={user_vm_sockaddr}, gateway={gateway_sockaddr})"
        );
        let mut linuxd_args: Vec<String> = vec![
            format!("{}/linuxd.elf", binary_directory),
            "-control-plane-addr".to_string(),
            control_plane_sockaddr.to_string(),
            "-user-vm-bind-addr".to_string(),
            user_vm_sockaddr.to_string(),
            "-gateway-bind-addr".to_string(),
            gateway_sockaddr.to_string(),
        ];
        if let Some(hwloc) = hwloc {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                hwloc.get_linuxd_core_str(),
            ];
            linuxd_args.splice(0..0, taskset);
        }

        let child = Command::new(&linuxd_args[0])
            .args(&linuxd_args[1..])
            .stdout(Stdio::piped())
            .spawn()?;

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
                if let Some(pid) = child.id() {
                    debug!("killing linuxd instance (pid={pid:?})");
                    let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
                }

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
        match control_plane::send_command(
            &mut self.control_plane_stream,
            control_plane::Command::Shutdown,
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
