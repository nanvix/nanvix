// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! User VM management for single-process mode.
//!
//! This module provides functionality to spawn and manage User VM instances as async tasks
//! within the same process. This mode simplifies testing and development by running all
//! components in a single process, making debugging and profiling easier.

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
use ::anyhow::Result;
use ::control_plane_api::{
    NanvixdCommand,
    NanvixdControlMessage,
};
use ::std::{
    mem,
    process::ExitCode,
    str::FromStr,
};
use ::sys::ipc::Message;
use ::syscomm::{
    SocketListener,
    SocketStream,
    SocketType,
    UnboundSocket,
    WriteAll,
};
use ::syslog::{
    debug,
    error,
    info,
    trace,
    warn,
};
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::timeout,
};
use ::user_vm_api::{
    NewUserVm,
    UserVmIdentifier,
    NEW_USER_VM_MESSAGE_LEN,
};
use ::uservm::{
    io_thread::IoThread,
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
    },
    UserVm as EmbeddedUserVm,
    CHANNEL_CAPACITY,
    CONTROL_PLANE_CONNECT_TIMEOUT,
    SYSTEM_VM_CONNECT_TIMEOUT,
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
    /// Underlying task.
    task: Option<JoinHandle<Result<ExitCode>>>,
    /// Control-plane socket stream.
    control_plane_stream: SocketStream,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl UserVm {
    ///
    /// # Description
    ///
    /// Spawns a new User VM instance as a task in the current process.
    ///
    /// # Parameters
    ///
    /// - `args`: User VM arguments.
    /// - `control_plane_listener`: Control-plane socket listener.
    ///
    /// # Returns
    ///
    /// On success, this function returns a handle to the spawned User VM instance. On failure,
    /// this function returns an error object instead.
    ///
    pub async fn spawn(
        args: &UserVmArgs,
        control_plane_listener: &mut SocketListener,
    ) -> Result<Self> {
        trace!("spawn(): args={args:?}");

        // Check if CPU affinity settings were provided.
        if let Some(hwloc) = args.hwloc() {
            warn!("spawn(): single-process mode ignores hwloc affinity settings (hwloc={hwloc:?})");
        }

        // Clone configuration values to move to User VM task.
        let control_plane_addr: String = args.control_plane_socket_info().0.clone();
        let system_vm_addr: String = args.system_vm_socket_info().0.clone();
        let gateway_sockaddr: String = args.gateway_socket_info().0.clone();
        let kernel_filename: String = args.kernel_binary_path().to_string();
        let initrd_filename: String = args.program().to_string();
        let initrd_args: Option<String> = args.program_args().map(|s| s.to_string());
        let stderr_file: Option<String> = args.console_file().map(|s| s.to_string());
        let user_vm_id: UserVmIdentifier = args.uservm_id();
        let control_plane_sockaddr_type: String =
            args.control_plane_socket_info().1.to_str().to_string();
        let system_vm_sockaddr_type: String = args.system_vm_socket_info().1.to_str().to_string();
        let gateway_sockaddr_type: String = args.gateway_socket_info().1.to_str().to_string();

        // Spawn the User VM as a new task.
        let uservm_task: JoinHandle<Result<ExitCode>> = ::tokio::spawn(async move {
            let (vcpu_thread_stdout_tx, io_thread_data_rx) =
                mpsc::channel::<Message>(CHANNEL_CAPACITY);
            let (io_thread_data_tx, memory_thread_data_rx) =
                mpsc::channel::<Message>(CHANNEL_CAPACITY);
            let (io_thread_control_tx, io_control_rx) =
                mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
            let (io_control_tx, io_thread_control_rx) =
                mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);

            // Connect to the control-plane socket.
            let unbound_socket: UnboundSocket =
                UnboundSocket::new(SocketType::from_str(&control_plane_sockaddr_type)?);
            let control_plane_stream: SocketStream = match timeout(
                CONTROL_PLANE_CONNECT_TIMEOUT,
                unbound_socket.connect(&control_plane_addr),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    debug!("user VM connected to control-plane (addr={control_plane_addr})");
                    stream
                },
                Ok(Err(e)) => {
                    let reason: String = format!(
                        "failed to connect to control plane (control_plane_addr={:?}, error={e:?})",
                        control_plane_addr
                    );
                    error!("spawn(): {reason}");
                    return Err(anyhow::anyhow!("{reason}"));
                },
                Err(_elapsed) => {
                    let reason: String = format!(
                        "timed out trying to connect to control plane (control_plane_addr={:?})",
                        control_plane_addr
                    );
                    error!("spawn(): {reason}");
                    return Err(anyhow::anyhow!("{reason}"));
                },
            };

            // Connect to the system VM socket.
            let unbound_socket: UnboundSocket =
                UnboundSocket::new(SocketType::from_str(&system_vm_sockaddr_type)?);
            let system_vm_stream: SocketStream =
                match timeout(SYSTEM_VM_CONNECT_TIMEOUT, unbound_socket.connect(&system_vm_addr))
                    .await
                {
                    Ok(Ok(mut stream)) => {
                        info!(
                            "spawn(): connected to system VM (system_vm_addr={:?})",
                            system_vm_addr
                        );
                        let new_msg: NewUserVm = match NewUserVm::new(
                            user_vm_id,
                            gateway_sockaddr.clone(),
                            SocketType::from_str(&gateway_sockaddr_type)?,
                        ) {
                            Ok(message) => message,
                            Err(e) => {
                                let reason: String = format!(
                                    "failed to construct user VM registration message \
                                     (error={e:?})"
                                );
                                error!("spawn(): {reason}");
                                return Err(anyhow::anyhow!(reason));
                            },
                        };

                        debug!("forwarding user vm information to system vm");
                        let new_msg_bytes: [u8; NEW_USER_VM_MESSAGE_LEN] = new_msg.to_bytes();
                        if let Err(e) = stream.write_all(&new_msg_bytes).await {
                            let reason: String = format!(
                                "failed to send user VM registration message (error={e:?})"
                            );
                            error!("spawn(): {reason}");
                            return Err(anyhow::anyhow!(reason));
                        }
                        stream
                    },
                    Ok(Err(e)) => {
                        let reason: String = format!(
                            "failed to connect to system VM (system_vm_addr={:?}, error={e:?})",
                            system_vm_addr
                        );
                        error!("spawn(): {reason}");
                        return Err(anyhow::anyhow!("{reason}"));
                    },
                    Err(_) => {
                        let reason: String = format!(
                            "timed out trying to connect to system VM (system_vm_addr={:?})",
                            system_vm_addr
                        );
                        error!("spawn(): {reason}");
                        return Err(anyhow::anyhow!("{reason}"));
                    },
                };

            // Spawn I/O thread.
            let io_thread: JoinHandle<Result<()>> = IoThread::spawn(
                system_vm_stream,
                io_thread_data_rx,
                io_thread_data_tx,
                io_thread_control_tx,
                io_thread_control_rx,
                control_plane_stream,
            )?;

            // Spawn VMM thread.
            let vmm_handle: JoinHandle<Result<u16>> = EmbeddedUserVm::spawn(::uservm::UserVmArgs {
                memory_size: ::config::kernel::MEMORY_SIZE,
                kernel_filename,
                initrd_filename: Some(initrd_filename.clone()),
                initrd_args,
                stderr: stderr_file,
                vcpu_thread_stdout_tx,
                memory_thread_data_rx,
                io_control_rx,
                io_control_tx,
            });

            // Wait for VMM thread to finish.
            let result: Result<ExitCode> = match vmm_handle.await? {
                Ok(exit_status) => {
                    if exit_status == 0 {
                        return Ok(ExitCode::from(0));
                    }
                    let exit_code_result: ::std::result::Result<u8, ::std::num::TryFromIntError> =
                        exit_status.try_into();
                    match exit_code_result {
                        Ok(code) => Ok(ExitCode::from(code)),
                        Err(_) => {
                            let reason: String = format!(
                                "failed to convert exit status (exit_status={exit_status})"
                            );
                            error!("spawn(): {reason}");
                            Err(anyhow::anyhow!(reason))
                        },
                    }
                },
                Err(error) => {
                    let reason: String = format!("virtual machine failed ({error:?})");
                    error!("spawn(): {reason}");
                    Err(anyhow::anyhow!(reason))
                },
            };

            // Wait for I/O thread to finish.
            if let Err(error) = io_thread.await? {
                error!("spawn(): I/O thread failed (error={error:?})");
                // Don't bail as we want to return the VM's exit code.
            }

            result
        });

        // Wait for the User VM task to connect to the control-plane socket.
        let control_plane_stream: SocketStream =
            match timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, control_plane_listener.accept()).await {
                Ok(Ok(stream)) => stream,
                Ok(Err(error)) => {
                    uservm_task.abort();
                    let reason: String = format!(
                        "error connecting control-plane to embedded user VM (error={error:?})"
                    );
                    error!("spawn(): {reason}");
                    anyhow::bail!("{reason}");
                },
                Err(elapsed) => {
                    uservm_task.abort();
                    let reason: String = format!(
                        "timed-out waiting for user VM to connect the control-plane stream \
                         (elapsed={elapsed:?})"
                    );
                    error!("spawn(): {reason}");
                    anyhow::bail!("{reason}");
                },
            };

        Ok(Self {
            task: Some(uservm_task),
            control_plane_stream,
        })
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

        // Wait for User VM to finish.
        if let Some(task) = self.task.take() {
            match timeout(CLEANUP_TIMEOUT, task).await {
                Ok(join_result) => match join_result {
                    Ok(Ok(exit_status)) => {
                        if exit_status != ExitCode::SUCCESS {
                            warn!(
                                "shutdown(): user VM returned with non-zero exit status \
                                 (code={exit_status:?})",
                            );
                        }
                    },
                    Ok(Err(error)) => {
                        warn!(
                            "shutdown(): embedded user VM terminated with error (error={error:?})"
                        );
                    },
                    Err(join_error) => {
                        warn!("shutdown(): embedded user VM task panicked (error={join_error:?})");
                    },
                },
                Err(elapsed) => {
                    warn!(
                        "shutdown(): timed-out waiting for embedded user VM to shutdown \
                         (error={elapsed:?})"
                    );
                },
            }
        }
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
        if let Some(task) = &self.task {
            !task.is_finished()
        } else {
            false
        }
    }
}
