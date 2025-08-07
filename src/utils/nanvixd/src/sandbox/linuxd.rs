// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config,
    control_plane,
};
use ::anyhow::Result;
use ::hwloc::HwLoc;
use ::std::process::Stdio;
use ::syscomm::{
    Socket,
    SocketStream,
    SocketType,
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
    pub fn spawn(control_plane_sockaddr: &str, user_vm_sockaddr: &str, gateway_sockaddr: &str, hwloc: Option<HwLoc>) -> Result<Self> {
        // Start the control-plane socket in listening mode.
        let control_plane_listener = match Socket::bind(SocketType::Unix, control_plane_sockaddr.to_string()) {
            Ok(listener) => listener,
            Err(e) => {
                error!("failed to bind control-plane listening socket (address={control_plane_sockaddr}, error={e:?})");
                return Err(anyhow::anyhow!("failed to bind control-plane listening socket"));
            }
        };

        debug!("spawning linux daemon (control-plane={control_plane_sockaddr}, user-vm={user_vm_sockaddr}, \
            gateway={gateway_sockaddr})");
        let mut linuxd_args: Vec<String> = vec![
            format!("{}/linuxd.elf", config::BINARY_DIRECTORY),
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
        let control_plane_stream = loop {
            match control_plane_listener.accept() {
                Ok(stream) => {
                    debug!("nanvixd received connection from linuxd's control-plane socket");
                    break stream;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(error) => {
                    error!("nanvixd failed to accept connection from linuxd's control-plane (error={error:?})");
                    continue;
                }
            }
        };

        Ok(Self {
            child,
            control_plane_stream,
        })
    }

    /// Send a shutdown message to linuxd so that it can clean-up its internal resources.
    pub async fn shutdown(&mut self) -> Result<()> {
        match control_plane::send_command(&mut self.control_plane_stream, control_plane::Command::Shutdown) {
            Ok(()) => {},
            Err(e) => {
                // FIXME: this send_command is expected to fail until support is implemented in
                // linuxd.
                error!("failed to send shutdown command to linuxd (error={e:?})");
            }
        };

        // FIXME: when linuxd reacts on shutdown commands, we will be able to get rid of this
        // manual kill.
        match self.child.id() {
            Some(pid) => {
                let ret_code = unsafe { libc::kill(pid as libc::pid_t, libc::SIGINT) };

                if ret_code < 0 {
                    let reason: String = format!("error sending SIGINT to linuxd: {}", std::io::Error::last_os_error());
                    error!("{reason}");

                    return Err(anyhow::anyhow!(reason));
                }
            },
            None => {
                let reason: String = "linuxd process has no PID".to_string();
                error!("{reason}");

                return Err(anyhow::anyhow!(reason));
            }
        }

        // Wait for linuxd instance to finish.
        match self.child.wait().await {
            Ok(exit_status) => {
                if !exit_status.success() {
                    let reason: String = format!("linuxd returned with non-zero exit status: {:?}", exit_status.code());
                    // FIXME: change this debug to error once linuxd dies gracefully after a
                    // SIGINT.
                    debug!("{reason}");

                    Err(anyhow::anyhow!(reason))
                } else {
                    Ok(())
                }
            }
            Err(e) => {
                let reason: String = format!("error waiting for linuxd: {e:?}");
                error!("{reason}");

                Err(anyhow::anyhow!(reason))
            }
        }
    }
}
