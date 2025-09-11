// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sandbox::{
    config::SandboxConfig,
    tag::SandboxTag,
};
use ::anyhow::Result;
use ::log::{
    debug,
    error,
};
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
    /// Configuration for this sandbox instance. It includes a RAII handle around the TCP
    /// port used for the gateway of this user VM if linuxd is deployed in an L2 VM.
    _config: SandboxConfig,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Microvm {
    pub async fn spawn(
        sandbox_tag: SandboxTag,
        sandbox_config: SandboxConfig,
        control_plane_listener: &mut SocketListener,
        control_plane_poll: &mut Poll,
    ) -> Result<Self> {
        let mut user_vm_args: Vec<String> = vec![
            format!("{}/microvm.elf", sandbox_config.binary_directory()),
            ::microvm::args::Args::OPT_LOGFILE.to_string(),
            ::microvm::args::Args::OPT_USER_VM_ID.to_string(),
            sandbox_tag.sandbox_id().to_string(),
            ::microvm::args::Args::OPT_KERNEL.to_string(),
            format!("{}/kernel.elf", sandbox_config.binary_directory()),
            ::microvm::args::Args::OPT_INITRD.to_string(),
            sandbox_config.program().to_string(),
            ::microvm::args::Args::OPT_SYSTEM_VM_SOCKADDR.to_string(),
            sandbox_config.user_vm_sockaddr().to_string(),
            ::microvm::args::Args::OPT_CONTROL_PLANE_SOCKADDR.to_string(),
            sandbox_config.control_plane_sockaddr().to_string(),
            ::microvm::args::Args::OPT_GATEWAY_SOCKADDR.to_string(),
            sandbox_config.gateway_sockaddr().to_string(),
        ];

        if sandbox_config.l2() {
            user_vm_args.push(::microvm::args::Args::OPT_SYSTEM_VM_SOCKET_TYPE.to_string());
            user_vm_args.push("tcp".to_string());
            user_vm_args.push(::microvm::args::Args::OPT_CONTROL_PLANE_SOCKET_TYPE.to_string());
            user_vm_args.push("tcp".to_string());
            user_vm_args.push(::microvm::args::Args::OPT_GATEWAY_SOCKET_TYPE.to_string());
            user_vm_args.push("tcp".to_string());
        }

        if let Some(program_args) = sandbox_config.program_args() {
            user_vm_args.push(::microvm::args::Args::OPT_INITRD_ARGS.to_string());
            user_vm_args.push(program_args.to_string());
        }

        if let Some(stderr_file) = sandbox_config.console_file() {
            user_vm_args.push(::microvm::args::Args::OPT_STDERR.to_string());
            user_vm_args.push(stderr_file.to_string());
        }

        if let Some(hwloc) = sandbox_config.hwloc() {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                hwloc.get_nanovm_core_str(),
            ];
            user_vm_args.splice(0..0, taskset);
        }

        // Inherit stdout/stderr so that errors when spawning the command are surfaced to nanvixd.
        let child = Command::new(&user_vm_args[0])
            .args(&user_vm_args[1..])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        debug!(
            "spawning microvm child.pid={:?} program={:?} args={:?} addr={:?} stderr={:?} l2={}",
            child.id(),
            sandbox_config.program(),
            sandbox_config.program_args(),
            sandbox_config.user_vm_sockaddr(),
            sandbox_config.console_file(),
            sandbox_config.l2(),
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
            addr: sandbox_config.user_vm_sockaddr().to_string(),
            control_plane_stream,
            _config: sandbox_config,
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
