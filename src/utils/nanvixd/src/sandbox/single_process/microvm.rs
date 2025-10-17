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
use ::config::syscomm::DEFAULT_CHANNEL_CAPACITY;
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
    trace,
    warn,
};
use ::tokio::{
    sync::{
        mpsc,
        Mutex,
    },
    task::JoinHandle,
    time::{
        timeout,
        Duration,
    },
};
use ::user_vm_api::{
    NewUserVm,
    UserVmIdentifier,
};
use ::uservm::{
    io_thread::IoThread,
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
    },
    UserVm,
    UserVmArgs,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Handle to a running MicroVM instance.
///
pub struct Microvm {
    /// Underlying task.
    ///
    task: Mutex<Option<JoinHandle<Result<ExitCode>>>>,
    control_plane_stream: SocketStream,
    /// Configuration for this sandbox instance. It includes a RAII handle around the TCP
    /// port used for the gateway of this user VM if linuxd is deployed in an L2 VM.
    _config: SandboxConfig,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Microvm {
    ///
    /// # Description
    ///
    /// Spawns a new MicroVM instance as a task in the current process.
    ///
    /// # Parameters
    ///
    /// - `sandbox_tag`: Sandbox tag.
    /// - `sandbox_config`: Sandbox configuration.
    /// - `control_plane_listener`: Control-plane socket listener.
    ///
    /// # Return Value
    ///
    /// On success this function returns a future that, when resolved, yields a handle to the
    /// spawned MicroVM instance. On failure, this function returns an error object instead.
    ///
    pub async fn spawn(
        sandbox_tag: SandboxTag,
        sandbox_config: SandboxConfig,
        control_plane_listener: &mut SocketListener,
    ) -> Result<Self> {
        trace!("spawn(): sandbox_tag={sandbox_tag:?}, sandbox_config={sandbox_config:?}");

        // Check if CPU affinity settings were provided.
        if let Some(hwloc) = sandbox_config.hwloc() {
            warn!("spawn(): single-process mode ignores hwloc affinity settings (hwloc={hwloc:?})");
        }

        // Check if L2 mode was requested.
        if sandbox_config.l2() {
            let reason: &str = "single-process mode does not support L2 deployments";
            error!("spawn(): {reason}");
            anyhow::bail!("{reason}");
        }

        // Clone configuration values to move to User VM task.
        let control_plane_addr: String = sandbox_config.control_plane_sockaddr().to_string();
        let user_vm_addr: String = sandbox_config.user_vm_sockaddr().to_string();
        let gateway_sockaddr: String = sandbox_config.gateway_sockaddr().to_string();
        let kernel_filename: String = format!("{}/kernel.elf", sandbox_config.binary_directory());
        let initrd_filename: String = sandbox_config.program().to_string();
        let initrd_args: Option<String> =
            sandbox_config.program_args().map(|args| args.to_string());
        let stderr_file: Option<String> =
            sandbox_config.console_file().map(|path| path.to_string());
        let user_vm_id: UserVmIdentifier = sandbox_tag.sandbox_id();
        let control_plane_sockaddr_type: String =
            sandbox_config.control_plane_sockaddr_type().to_string();
        let system_vm_sockaddr_type: String = sandbox_config.system_vm_sockaddr_type().to_string();
        let gateway_sockaddr_type: String = sandbox_config.gateway_sockaddr_type().to_string();

        // Spawn the User VM as a new task.
        let uservm_task: JoinHandle<Result<ExitCode>> = ::tokio::spawn(async move {
            let (vcpu_thread_stdout_tx, io_thread_data_rx) =
                mpsc::channel::<Message>(DEFAULT_CHANNEL_CAPACITY);
            let (io_thread_data_tx, memory_thread_data_rx) =
                mpsc::channel::<Message>(DEFAULT_CHANNEL_CAPACITY);
            let (io_thread_control_tx, io_control_rx) =
                mpsc::channel::<IoControlCommand>(DEFAULT_CHANNEL_CAPACITY);
            let (io_control_tx, io_thread_control_rx) =
                mpsc::channel::<IoControlResponse>(DEFAULT_CHANNEL_CAPACITY);

            // Connect to the control-plane socket.
            let control_plane_stream: SocketStream =
                match UnboundSocket::new(SocketType::from_str(&control_plane_sockaddr_type)?)
                    .connect(control_plane_addr.clone())
                    .await
                {
                    Ok(stream) => {
                        debug!(
                            "embedded user VM connected to control-plane \
                             (addr={control_plane_addr})"
                        );
                        stream
                    },
                    Err(e) => {
                        let reason = format!(
                            "failed to connect embedded user VM to control-plane \
                             (addr={control_plane_addr}, error={e:?})"
                        );
                        error!("spawn(): {reason}");
                        anyhow::bail!(reason);
                    },
                };

            // Connect to the system VM socket.
            let mut system_vm_stream: SocketStream =
                match UnboundSocket::new(SocketType::from_str(&system_vm_sockaddr_type)?)
                    .connect(user_vm_addr.clone())
                    .await
                {
                    Ok(stream) => {
                        debug!("embedded user VM connected to linuxd (addr={user_vm_addr})");
                        stream
                    },
                    Err(error) => {
                        let reason = format!(
                            "failed to connect embedded user VM to linuxd (addr={user_vm_addr}, \
                             error={error:?})"
                        );
                        error!("spawn(): {reason}");
                        anyhow::bail!(reason);
                    },
                };

            // Send NewUserVm registration to linuxd.
            let new_msg: NewUserVm = match NewUserVm::new(
                user_vm_id,
                gateway_sockaddr.clone(),
                SocketType::from_str(&gateway_sockaddr_type)?,
            ) {
                Ok(message) => message,
                Err(error) => {
                    let reason: String = format!(
                        "failed to create embedded user VM registration message \
                         (user_vm_id={user_vm_id}, gateway_sockaddr={gateway_sockaddr}, \
                         error={error:?})"
                    );
                    error!("spawn(): {reason}");
                    anyhow::bail!(reason);
                },
            };
            debug!("forwarding embedded user VM registration to linuxd");
            let new_msg_bytes: [u8; ::user_vm_api::NEW_USER_VM_MESSAGE_LEN] = new_msg.to_bytes();
            if let Err(e) = system_vm_stream.write_all(&new_msg_bytes).await {
                let reason: String = format!(
                    "failed to forward embedded user VM registration to linuxd (error={e:?})"
                );
                error!("spawn(): {reason}");
                anyhow::bail!(reason);
            }

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
            let vmm_handle: JoinHandle<Result<u16>> = UserVm::spawn(UserVmArgs {
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
                Ok(0) => Ok(ExitCode::from(0)),
                Ok(exit_status) if exit_status != 0 => {
                    let exit_code_result: ::std::result::Result<u8, ::std::num::TryFromIntError> =
                        exit_status.try_into();
                    match exit_code_result {
                        Ok(code) => Ok(ExitCode::from(code)),
                        Err(_) => {
                            let reason: String = format!(
                                "failed to convert exit status (exit_status={exit_status})"
                            );
                            error!("main(): {reason}");
                            Err(anyhow::anyhow!(reason))
                        },
                    }
                },
                _ => {
                    let reason: String = "virtual machine failed".to_string();
                    error!("main(): {reason}");
                    Err(anyhow::anyhow!(reason))
                },
            };

            // Wait for I/O thread to finish.
            if let Err(error) = io_thread.await? {
                error!("main(): I/O thread failed (error={error:?})");
                // Don't bail as we want to return the VM's exit code.
            }

            result
        });

        // Wait for the User VM task to connect to the control-plane socket.
        let control_plane_stream: SocketStream = match timeout(
            Duration::from_secs(config::syscomm::ACCEPT_TIMEOUT_SECS),
            control_plane_listener.accept(),
        )
        .await
        {
            Ok(Ok(stream)) => stream,
            Ok(Err(error)) => {
                uservm_task.abort();
                let reason: String =
                    format!("error connecting control-plane to embedded user VM (error={error:?})");
                error!("spawn(): {reason}");
                anyhow::bail!("{reason}");
            },
            Err(elapsed) => {
                uservm_task.abort();
                let reason: String = format!(
                    "timed-out waiting for embedded user VM to connect the control-plane stream \
                     (error={elapsed:?})"
                );
                error!("spawn(): {reason}");
                anyhow::bail!("{reason}");
            },
        };

        Ok(Self {
            task: Mutex::new(Some(uservm_task)),
            control_plane_stream,
            _config: sandbox_config,
        })
    }

    ///
    /// # Description
    ///
    /// Shuts down the MicroVM instance.
    ///
    /// # Return Value
    ///
    /// On success this function returns a future that, when resolved, indicates that the MicroVM
    /// has been shutdown. On failure, this function returns an error object instead.
    ///
    pub async fn shutdown(&mut self) -> Result<()> {
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
            let reason: String =
                format!("failed to send shutdown command to embedded user VM (error={e:?})");
            error!("shutdown(): {reason}");
            return Err(anyhow::anyhow!("{reason}"));
        }

        // Wait for User VM to finish.
        // NOTE: Don't bail out if we fail to keep same behavior as multi-process implementation.
        if let Some(task) = self.task.lock().await.take() {
            match timeout(crate::config::CLEANUP_TIMEOUT, task).await {
                Ok(join_result) => match join_result {
                    Ok(Ok(exit_status)) => {
                        if exit_status != ExitCode::SUCCESS {
                            error!(
                                "shutdown(): user VM returned with non-zero exit status \
                                 (code={exit_status:?})",
                            );
                        }
                    },
                    Ok(Err(error)) => {
                        error!(
                            "shutdown(): embedded user VM terminated with error (error={error:?})"
                        );
                    },
                    Err(join_error) => {
                        error!("shutdown(): embedded user VM task panicked (error={join_error:?})");
                    },
                },
                Err(elapsed) => {
                    error!(
                        "shutdown(): timed-out waiting for embedded user VM to shutdown \
                         (error={elapsed:?})"
                    );
                },
            }
        }

        Ok(())
    }
}
