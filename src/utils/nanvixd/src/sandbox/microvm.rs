// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::hwloc::HwLoc;
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

pub struct Microvm {
    child: Option<Child>,
    addr: String,
    #[allow(dead_code)]
    // FIXME: the micro VM still does not support processing messages from the control-plane.
    control_plane_stream: SocketStream,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Microvm {
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        program: &str,
        program_args: Option<&str>,
        addr: &str,
        control_plane_addr: &str,
        stderr: Option<&str>,
        hwloc: Option<HwLoc>,
        binary_directory: &str,
        control_plane_listener: &mut SocketListener,
        control_plane_poll: &mut Poll,
    ) -> Result<Self> {
        let mut user_vm_args: Vec<String> = vec![
            format!("{}/microvm.elf", binary_directory),
            "-log-to-file".to_string(),
            "-kernel".to_string(),
            format!("{}/kernel.elf", binary_directory),
            "-initrd".to_string(),
            program.to_string(),
            "-system-vm-addr".to_string(),
            addr.to_string(),
            "-control-plane-addr".to_string(),
            control_plane_addr.to_string(),
        ];

        if let Some(program_args) = program_args {
            user_vm_args.push("-initrd_args".to_string());
            user_vm_args.push(program_args.to_string());
        }

        if let Some(stderr_file) = stderr {
            user_vm_args.push("-stderr".to_string());
            user_vm_args.push(stderr_file.to_string());
        }

        if let Some(hwloc) = hwloc {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                hwloc.get_nanovm_core_str(),
            ];
            user_vm_args.splice(0..0, taskset);
        }

        let child = Command::new(&user_vm_args[0])
            .args(&user_vm_args[1..])
            .stdout(Stdio::piped())
            .spawn()?;

        debug!(
            "spawning microvm child.pid={:?} program={:?} args={:?} addr={:?} stderr={:?}",
            child.id(),
            program,
            program_args,
            addr,
            stderr
        );

        // After the user VM has started, accept the incoming connection for the control-plane.
        // Post-condition: once the connection has been accepted, the user VM has been able to
        // connect to the system VM (if an address is provided).
        let control_plane_stream: SocketStream = match control_plane_listener.accept_timeout(
            control_plane_poll,
            Duration::from_secs(config::syscomm::ACCEPT_TIMEOUT_SECS),
        ) {
            Ok(stream) => stream,
            Err(e) => {
                // If the user VM has not accepted the control-plane connection, it means that
                // something went wrong during start-up. We kill the process ignoring errors,
                // and return an error.
                let reason: String =
                    format!("error connecting control-plane to user VM (error={e:?})");
                error!("{reason}");

                // Use a SIGKILL because the process is already faulty.
                if let Some(pid) = child.id() {
                    debug!("killing user VM instance (pid={pid:?})");
                    let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
                }

                return Err(anyhow::anyhow!("{reason}"));
            },
        };
        debug!("nanvixd received connection from the user VM's control-plane socket");

        Ok(Self {
            child: Some(child),
            addr: addr.to_string(),
            control_plane_stream,
        })
    }
}

impl Drop for Microvm {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            match child.id() {
                Some(pid) => {
                    let ret_code = unsafe { libc::kill(pid as libc::pid_t, libc::SIGINT) };

                    if ret_code < 0 {
                        error!(
                            "error sending SIGINT to user VM: {}",
                            std::io::Error::last_os_error()
                        );
                    }
                },
                None => error!("user VM process has no PID"),
            }

            // Wait for the child to finish in a separate thread to be able to `await` on it.
            tokio::spawn(async move {
                if let Err(e) = child.wait().await {
                    error!("failed to wait for user VM to shut down (error={e:?})");
                }
            });

            // Clean-up the per-micro VM socket file.
            // FIXME: this won't be necessary once all micro VMs share a socket in one linuxd
            // instance.
            match std::fs::remove_file(self.addr.clone()) {
                Ok(_) => {},
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
                Err(e) => {
                    error!("error removing micro VM socket (addr={}, error={e:?})", self.addr)
                },
            }
        }
    }
}
