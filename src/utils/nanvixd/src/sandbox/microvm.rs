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
use ::mio::Poll;
use ::std::{
    process::Stdio,
    time::Duration,
};
use ::syscomm::{
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

//==================================================================================================
// Structures
//==================================================================================================

pub struct Microvm {
    child: Option<Child>,
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

        // Before returning, we must make sure that the gateway listener socket in linuxd is ready
        // to accept connections. The way we do so, is by actually connecting once to it, and
        // ignoring the resulting stream.
        let socket_type: SocketType = if sandbox_config.l2() {
            SocketType::Tcp
        } else {
            SocketType::Unix
        };
        SocketStream::connect_timeout(
            socket_type,
            sandbox_config.gateway_sockaddr().to_string(),
            Duration::from_secs(config::syscomm::CONNECT_TIMEOUT_SECS),
        )
        .map_err(|e| {
            let reason: String = format!(
                "error establishing throw-away connection to gateway socket (addr={}, error={e:?})",
                sandbox_config.gateway_sockaddr()
            );
            error!("{reason}");
            anyhow::anyhow!(reason)
        })?;
        debug!(
            "nanvixd established throw-away gateway connection (addr={})",
            sandbox_config.gateway_sockaddr()
        );

        Ok(Self {
            child: Some(child),
            control_plane_stream,
            _config: sandbox_config,
        })
    }

    ///
    /// # Description
    ///
    /// Send a shutdown message to the user VM's control-plane socket so that it can gracefully
    /// shutdown, and wait until the process dies.
    ///
    pub async fn shutdown(&mut self) -> Result<()> {
        match control_plane_api::send_command(
            &mut self.control_plane_stream,
            control_plane_api::Command::Shutdown,
        ) {
            Ok(()) => {},
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                debug!("user VM already shut down");
            },
            Err(e) => {
                error!("failed to send shutdown command to user VM (error={e:?})");
            },
        };

        // Wait for user VM instance to finish.
        if let Some(mut child) = self.child.take() {
            match child.wait().await {
                Ok(exit_status) => {
                    if !exit_status.success() {
                        error!(
                            "user VM returned with non-zero exit status (code={:?})",
                            exit_status.code()
                        );
                    }
                },
                Err(e) => {
                    error!("error waiting for user VM (error={e:?})");
                },
            }
        }

        Ok(())
    }
}
