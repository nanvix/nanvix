// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! User VM management for multi-process mode.
//!
//! This module provides functionality to spawn and manage User VM instances as separate
//! processes. It handles process lifecycle, control-plane communication, gateway sockets,
//! and supports L2 deployment with TCP port allocation.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::{
        CLEANUP_TIMEOUT,
        CONTROL_PLANE_ACCEPT_TIMEOUT,
    },
    UserVmArgs,
};
#[cfg(not(feature = "single-process"))]
use crate::{
    netns::NetnsHandle,
    netns_exec::command_in_netns,
};
use ::anyhow::Result;
use ::control_plane_api::{
    NanvixdCommand,
    NanvixdControlMessage,
};
use ::std::{
    mem,
    process::{
        ExitStatus,
        Stdio,
    },
};
use ::syscomm::{
    SocketListener,
    SocketStream,
    WriteAll,
};
use ::log::{
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

///
/// # Description
///
/// Handle to a running User VM instance.
///
pub struct UserVm {
    /// Child process handle.
    child: Option<Child>,
    /// Control-plane socket stream.
    control_plane_stream: SocketStream,
    /// Optional RAII handle to the network namespace the user VM is spawned in. Even if unused, we
    /// tie its lifecycle to the user VM.
    #[cfg(not(feature = "single-process"))]
    _netns_handle: Option<NetnsHandle>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl UserVm {
    ///
    /// # Description
    ///
    /// Spawns a new User VM instance as a separate process.
    ///
    /// # Parameters
    ///
    /// - `args`: User VM arguments.
    /// - `control_plane_listener`: Control-plane socket listener.
    /// - `netns_handle`: Optional handle to a network namespace (L2-mode only).
    ///
    /// # Returns
    ///
    /// On success, this function returns a handle to the spawned User VM instance. On failure,
    /// this function returns an error object instead.
    ///
    pub async fn spawn(
        args: &UserVmArgs,
        control_plane_listener: &mut SocketListener,
        #[cfg(not(feature = "single-process"))] netns_handle: Option<NetnsHandle>,
    ) -> Result<Self> {
        trace!("spawn(): args={args:?}");

        let mut user_vm_args: Vec<String> = vec![
            args.uservm_binary_path().to_string(),
            ::uservm::args::Args::OPT_LOGFILE.to_string(),
            ::uservm::args::Args::OPT_LOGDIR.to_string(),
            args.log_directory().to_string(),
            ::uservm::args::Args::OPT_USER_VM_ID.to_string(),
            args.uservm_id().to_string(),
            ::uservm::args::Args::OPT_KERNEL.to_string(),
            args.kernel_binary_path().to_string(),
            ::uservm::args::Args::OPT_INITRD.to_string(),
            args.program().to_string(),
            ::uservm::args::Args::OPT_SYSTEM_VM_SOCKADDR.to_string(),
            args.system_vm_socket_info().0.to_string(),
            ::uservm::args::Args::OPT_SYSTEM_VM_SOCKET_TYPE.to_string(),
            args.system_vm_socket_info().1.to_str().to_string(),
            ::uservm::args::Args::OPT_CONTROL_PLANE_SOCKADDR.to_string(),
            args.control_plane_connect_socket_info().0.to_string(),
            ::uservm::args::Args::OPT_CONTROL_PLANE_SOCKET_TYPE.to_string(),
            args.control_plane_connect_socket_info()
                .1
                .to_str()
                .to_string(),
            ::uservm::args::Args::OPT_GATEWAY_SOCKADDR.to_string(),
            args.gateway_socket_info().0.to_string(),
            ::uservm::args::Args::OPT_GATEWAY_SOCKET_TYPE.to_string(),
            args.gateway_socket_info().1.to_str().to_string(),
        ];

        if let Some(program_args) = args.program_args() {
            user_vm_args.push(::uservm::args::Args::OPT_INITRD_ARGS.to_string());
            user_vm_args.push(program_args.to_string());
        }

        if let Some(ramfs_filename) = args.ramfs_filename() {
            user_vm_args.push(::uservm::args::Args::OPT_RAMFS.to_string());
            user_vm_args.push(ramfs_filename.to_string());
        }

        if let Some(stderr_file) = args.console_file() {
            user_vm_args.push(::uservm::args::Args::OPT_STDERR.to_string());
            user_vm_args.push(stderr_file.to_string());
        }

        if let Some(hwloc) = args.hwloc() {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                hwloc.get_nanovm_core_str(),
            ];
            user_vm_args.splice(0..0, taskset);
        }

        debug!("spawning uservm (program={:?} args={:?})", args.program(), user_vm_args,);
        let mut cmd: Command = {
            // In an L2-deployment, spawn the user VM inside a network namespace.
            #[cfg(not(feature = "single-process"))]
            if let Some(netns_handle) = &netns_handle {
                command_in_netns(&netns_handle.netns_info()?, &user_vm_args[0], &user_vm_args[1..])
            } else {
                let mut cmd: Command = Command::new(&user_vm_args[0]);
                cmd.args(&user_vm_args[1..]);
                // Ensure the child process is killed if the Child handle is dropped without
                // explicit cleanup. This acts as a best-effort safety net during normal unwinding
                // and shutdown paths where drop handlers run, helping to prevent orphaned
                // processes.
                cmd.kill_on_drop(true);

                cmd
            }

            #[cfg(feature = "single-process")]
            {
                let mut cmd: Command = Command::new(&user_vm_args[0]);
                cmd.args(&user_vm_args[1..]);
                // Ensure the child process is killed if the Child handle is dropped without
                // explicit cleanup.  This acts as a best-effort safety net during normal unwinding
                // and shutdown paths where drop handlers run, helping to prevent orphaned
                // processes.
                cmd.kill_on_drop(true);
                cmd
            }
        };

        // Inherit stdout/stderr so that errors when spawning the command are surfaced to nanvixd.
        let child: Child = cmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        debug!(
            "spawning uservm child.pid={:?} program={:?} args={:?} addr={:?} stderr={:?}",
            child.id(),
            args.program(),
            args.program_args(),
            args.system_vm_socket_info(),
            args.console_file(),
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
            #[cfg(not(feature = "single-process"))]
            _netns_handle: netns_handle,
        })
    }

    ///
    /// # Description
    ///
    /// Helper method to send a SIGKILL to the user VM process in case it is faulty and we need to
    /// clean-up.
    ///
    /// # Parameters
    ///
    /// - `child`: The user VM process handle.
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
    /// # Returns
    ///
    /// Returns `Some(ExitStatus)` if the child process finished before the cleanup timeout.
    /// Returns `None` otherwise.
    ///
    pub async fn shutdown(&mut self) -> Option<ExitStatus> {
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
            warn!("shutdown(): failed to send shutdown command to user VM (error={e:?})");
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

                    return Some(exit_status);
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

        None
    }

    ///
    /// # Description
    ///
    /// Checks if the User VM instance is still running.
    ///
    /// # Returns
    ///
    /// This function returns true if the target User VM is still running, and false otherwise.
    ///
    pub fn is_running(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(_status)) => false,
                Ok(None) => true,
                Err(e) => {
                    warn!("is_running(): failed to query user VM status (error={e:?})");
                    false
                },
            }
        } else {
            false
        }
    }
}
