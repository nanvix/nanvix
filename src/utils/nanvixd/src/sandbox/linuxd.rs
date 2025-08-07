// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config;
use ::anyhow::Result;
use ::hwloc::HwLoc;
use ::std::{
    fs,
    io::ErrorKind,
    process::Stdio,
};
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
    // TODO: we currently do not send any information via the control-plane stream.
    _control_plane_stream: SocketStream,
    // Linuxd currently does not properly clean-up on shutdown (there is no such thing as shutdown)
    // so we need to make sure we clean-up the linuxd socket when we drop linuxd. We thus keep
    // track of it here.
    linuxd_sockaddr: String,
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
                Err(error) => {
                    error!("nanvixd failed to accept connection from linuxd's control-plane (error={error:?})");
                    continue;
                }
            }
        };

        Ok(Self {
            child,
            _control_plane_stream: control_plane_stream,
            linuxd_sockaddr: user_vm_sockaddr.to_string(),
        })
    }
}

impl Drop for LinuxDaemon {
    fn drop(&mut self) {
        match self.child.id() {
            Some(pid) => {
                let ret_code = unsafe { libc::kill(pid as libc::pid_t, libc::SIGINT) };

                if ret_code < 0 {
                    error!("error sending SIGINT to linuxd: {}", std::io::Error::last_os_error());
                }
            },
            None => error!("linuxd process has no PID"),
        }

        // Clean-up the socket files left behind by linuxd.
        match fs::remove_file(self.linuxd_sockaddr.clone()) {
            Ok(_) => {},
            Err(ref e) if e.kind() == ErrorKind::NotFound => {},
            Err(e) => error!("error removing UNIX socket (path={}, error={e:?})", self.linuxd_sockaddr.clone()),
        }
    }
}
