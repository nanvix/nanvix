// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::{
        CLEANUP_TIMEOUT,
        CONTROL_PLANE_ACCEPT_TIMEOUT,
    },
    sandbox::{
        config::SandboxConfig,
        tag::SandboxTag,
    },
};
use ::anyhow::Result;
use ::control_plane_api::{
    NanvixdCommand,
    NanvixdControlMessage,
};
use ::std::{
    mem,
    process::Stdio,
};
use ::syscomm::{
    SocketListener,
    SocketStream,
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

//==================================================================================================
// Structures
//==================================================================================================

pub struct UserVm {
    child: Option<Child>,
    control_plane_stream: SocketStream,
    /// Configuration for this sandbox instance. It includes a RAII handle around the TCP
    /// port used for the gateway of this user VM if linuxd is deployed in an L2 VM.
    _config: SandboxConfig,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl UserVm {
    pub async fn spawn(
        sandbox_tag: SandboxTag,
        sandbox_config: SandboxConfig,
        control_plane_listener: &mut SocketListener,
    ) -> Result<Self> {
        trace!("spawn(): sandbox_tag={sandbox_tag:?}, sandbox_config={sandbox_config:?}");

        let mut user_vm_args: Vec<String> = vec![
            format!("{}/uservm.elf", sandbox_config.binary_directory()),
            ::uservm::args::Args::OPT_LOGFILE.to_string(),
            ::uservm::args::Args::OPT_LOGDIR.to_string(),
            sandbox_config.log_directory().to_string(),
            ::uservm::args::Args::OPT_USER_VM_ID.to_string(),
            sandbox_tag.sandbox_id().to_string(),
            ::uservm::args::Args::OPT_KERNEL.to_string(),
            format!("{}/kernel.elf", sandbox_config.binary_directory()),
            ::uservm::args::Args::OPT_INITRD.to_string(),
            sandbox_config.program().to_string(),
            ::uservm::args::Args::OPT_SYSTEM_VM_SOCKADDR.to_string(),
            sandbox_config.user_vm_sockaddr().to_string(),
            ::uservm::args::Args::OPT_SYSTEM_VM_SOCKET_TYPE.to_string(),
            sandbox_config.system_vm_sockaddr_type().to_string(),
            ::uservm::args::Args::OPT_CONTROL_PLANE_SOCKADDR.to_string(),
            sandbox_config.control_plane_sockaddr().to_string(),
            ::uservm::args::Args::OPT_CONTROL_PLANE_SOCKET_TYPE.to_string(),
            sandbox_config.control_plane_sockaddr_type().to_string(),
            ::uservm::args::Args::OPT_GATEWAY_SOCKADDR.to_string(),
            sandbox_config.gateway_sockaddr().to_string(),
            ::uservm::args::Args::OPT_GATEWAY_SOCKET_TYPE.to_string(),
            sandbox_config.gateway_sockaddr_type().to_string(),
        ];

        debug!("spawning uservm (program={:?} args={:?})", sandbox_config.program(), user_vm_args,);

        if sandbox_config.l2() {
            user_vm_args.push(::uservm::args::Args::OPT_SYSTEM_VM_SOCKET_TYPE.to_string());
            user_vm_args.push("tcp".to_string());
            user_vm_args.push(::uservm::args::Args::OPT_CONTROL_PLANE_SOCKET_TYPE.to_string());
            user_vm_args.push("tcp".to_string());
            user_vm_args.push(::uservm::args::Args::OPT_GATEWAY_SOCKET_TYPE.to_string());
            user_vm_args.push("tcp".to_string());
        }

        if let Some(program_args) = sandbox_config.program_args() {
            user_vm_args.push(::uservm::args::Args::OPT_INITRD_ARGS.to_string());
            user_vm_args.push(program_args.to_string());
        }

        if let Some(stderr_file) = sandbox_config.console_file() {
            user_vm_args.push(::uservm::args::Args::OPT_STDERR.to_string());
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
            "spawning uservm child.pid={:?} program={:?} args={:?} addr={:?} stderr={:?} l2={}",
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
        let control_plane_stream: SocketStream =
            match timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, control_plane_listener.accept()).await {
                Ok(Ok(stream)) => stream,
                Ok(Err(e)) => {
                    // If the user VM has not accepted the control-plane connection, it means that
                    // something went wrong during start-up. We kill the process ignoring errors,
                    // and return an error.
                    let reason: String =
                        format!("error connecting control-plane to user VM (error={e:?})");
                    error!("{reason}");

                    Self::send_sigkill_to_child(child);

                    return Err(anyhow::anyhow!("{reason}"));
                },
                Err(e) => {
                    let reason: String = format!(
                        "timed-out waiting for user VM to connect the control-plane stream \
                         (error={e:?})"
                    );
                    error!("{reason}");

                    Self::send_sigkill_to_child(child);

                    return Err(anyhow::anyhow!("{reason}"));
                },
            };
        debug!("nanvixd received connection from the user VM's control-plane socket");

        Ok(Self {
            child: Some(child),
            control_plane_stream,
            _config: sandbox_config,
        })
    }

    ///
    /// # Description
    ///
    /// Helper method to send a SIGKILL to the user VM process in case it is faulty and we need to
    /// clean-up.
    ///
    /// # Arguments
    ///
    /// - `child`: the user VM process handle.
    ///
    fn send_sigkill_to_child(child: Child) {
        if let Some(pid) = child.id() {
            debug!("killing linuxd instance (pid={pid:?})");
            let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        }
    }

    ///
    /// # Description
    ///
    /// Shuts down the User VM instance.
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

        // Send shutdown command to User VM.
        if let Err(e) = self.control_plane_stream.write_all(&msg_bytes).await {
            warn!("shutdown(): failed to send shutdown command to embedded user VM (error={e:?})");
        }

        // Wait for user VM instance to finish.
        if let Some(mut child) = self.child.take() {
            match timeout(CLEANUP_TIMEOUT, child.wait()).await {
                Ok(Ok(exit_status)) => {
                    if !exit_status.success() {
                        warn!(
                            "shutdown(): user VM returned with non-zero exit status (code={:?})",
                            exit_status.code()
                        );
                    }
                },
                // If we encounter any errors while waiting for the user VM to gracefully shutdown,
                // make sure we kill the underlying instance.
                Ok(Err(error)) => {
                    warn!("shutdown(): user VM terminated with error (error={error:?})");
                    Self::send_sigkill_to_child(child);
                },
                Err(elapsed) => {
                    warn!(
                        "shutdown(): timed-out waiting for user VM to shutdown (error={elapsed:?})"
                    );
                    Self::send_sigkill_to_child(child);
                },
            }
        }
    }
}
