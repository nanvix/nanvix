// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! User VM management for standalone mode.
//!
//! This module provides functionality to spawn and manage User VM instances in standalone mode,
//! where the VM runs without connecting to a system VM, control-plane, or gateway. The VM's
//! stdout messages are drained and discarded to prevent back-pressure, while stderr is captured
//! as usual. This mode mirrors the `run_standalone` path in the `uservm` binary and is useful
//! for debugging and local testing.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::CLEANUP_TIMEOUT,
    UserVmArgs,
};
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::std::{
    convert::TryInto,
    os::unix::process::ExitStatusExt,
    process::ExitStatus,
};
use ::sys::ipc::IkcFrame;
use ::tokio::{
    runtime::Handle,
    sync::mpsc,
    task::{
        self,
        JoinHandle,
    },
    time::timeout,
};
use ::uservm::{
    counters::MessageCounters,
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
    },
    UserVm as EmbeddedUserVm,
    CHANNEL_CAPACITY,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Handle to a running User VM instance in standalone mode.
///
/// In standalone mode, the VM runs without any external connections (no system VM,
/// control-plane, or gateway). The VM's stdout channel is drained and discarded to prevent
/// back-pressure from blocking the VM.
///
pub struct UserVm {
    /// Underlying task running the VM and its drain loop.
    task: Option<JoinHandle<Result<u8>>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl UserVm {
    ///
    /// # Description
    ///
    /// Spawns a new User VM instance in standalone mode.
    ///
    /// This creates isolated channels (no external I/O thread) and spawns the VM in a blocking
    /// task. A drain task consumes and discards all VM stdout messages to prevent the channel
    /// from blocking the VM.
    ///
    /// # Parameters
    ///
    /// - `args`: User VM arguments providing kernel path, guest binary, and other configuration.
    ///
    /// # Returns
    ///
    /// On success, returns a handle to the spawned User VM instance. On failure, returns an
    /// error.
    ///
    pub async fn spawn(args: &UserVmArgs) -> Result<Self> {
        trace!("spawn(): args={args:?}");

        // Check if CPU affinity settings were provided.
        if let Some(hwloc) = args.hwloc() {
            warn!("spawn(): standalone mode ignores hwloc affinity settings (hwloc={hwloc:?})");
        }

        // Clone configuration values to move into the VM task.
        let kernel_filename: String = args.kernel_binary_path().to_string();
        let initrd_filename: String = args.program().to_string();
        let initrd_args: Option<String> = args.program_args().map(|s| s.to_string());
        let ramfs_filename: Option<String> = args.ramfs_filename().map(|s| s.to_string());
        let stderr_file: Option<String> = args.console_file().map(|s| s.to_string());

        // Spawn the User VM as a blocking task.
        let uservm_task: JoinHandle<Result<u8>> = task::spawn_blocking(move || {
            Handle::current().block_on(async move {
                // Create channels. In standalone mode these are wired directly without an I/O
                // thread. Nobody sends inbound data; the senders are kept alive so that the
                // receivers do not see an immediate channel close.
                let (vcpu_thread_stdout_tx, mut standalone_data_rx) =
                    mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
                let (_inbound_data_tx, memory_thread_data_rx) =
                    mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
                let (_io_cmd_tx, io_control_rx) =
                    mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
                let (io_control_tx, _io_resp_rx) =
                    mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);

                let counters: MessageCounters = MessageCounters::default();

                // Spawn the VMM.
                let vmm_handle: JoinHandle<Result<u16>> =
                    EmbeddedUserVm::spawn(::uservm::UserVmArgs {
                        kernel_filename,
                        initrd_filename: Some(initrd_filename.clone()),
                        initrd_args,
                        ramfs_filename,
                        stderr: stderr_file,
                        vcpu_thread_stdout_tx,
                        memory_thread_data_rx,
                        io_control_rx,
                        io_control_tx,
                        counters,
                        snapshot_path: None,
                        #[cfg(feature = "gdb")]
                        gdb_port: None,
                    });

                // Drain the VM's stdout channel. In standalone mode there is no system VM to
                // forward messages to, so we simply consume and discard them to prevent the
                // channel from blocking the VM.
                let drain_handle: JoinHandle<()> = tokio::spawn(async move {
                    while let Some(_msg) = standalone_data_rx.recv().await {}
                    debug!("spawn(): standalone mode: VM stdout channel closed");
                });

                // Wait for the VMM to finish.
                let vm_exit_status: Result<u16> = vmm_handle.await?;
                debug!("spawn(): uservm completed (exit_status={vm_exit_status:?})");

                // Wait for the drain task to finish.
                if let Err(error) = drain_handle.await {
                    warn!("spawn(): standalone drain task failed (error={error:?})");
                }

                // Convert the exit status.
                match vm_exit_status {
                    Ok(0) => Ok(0),
                    Ok(exit_status) => {
                        let exit_code_result: ::std::result::Result<
                            u8,
                            ::std::num::TryFromIntError,
                        > = exit_status.try_into();
                        match exit_code_result {
                            Ok(code) => Ok(code),
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
                }
            })
        });

        Ok(Self {
            task: Some(uservm_task),
        })
    }

    ///
    /// # Description
    ///
    /// Shuts down the User VM instance.
    ///
    /// In standalone mode there is no control-plane connection to send a shutdown command.
    /// This method waits for the VM task to finish within the cleanup timeout.
    ///
    /// # Returns
    ///
    /// On successful shutdown, returns the exit status of the User VM. Returns `None` if the
    /// task panicked, failed, or timed out.
    ///
    pub async fn shutdown(&mut self) -> Option<ExitStatus> {
        trace!("shutdown()");

        if let Some(task) = self.task.take() {
            match timeout(CLEANUP_TIMEOUT, task).await {
                Ok(join_result) => match join_result {
                    Ok(Ok(raw_code)) => {
                        return Some(exit_status_from_exit_code(raw_code));
                    },
                    Ok(Err(error)) => {
                        warn!("shutdown(): user VM terminated with error (error={error:?})");
                    },
                    Err(join_error) => {
                        warn!("shutdown(): user VM task panicked (error={join_error:?})");
                    },
                },
                Err(elapsed) => {
                    warn!(
                        "shutdown(): timed out waiting for user VM to shutdown (error={elapsed:?})"
                    );
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
    /// Returns `true` if the User VM task is still running, `false` otherwise.
    ///
    pub fn is_running(&mut self) -> bool {
        if let Some(task) = &self.task {
            !task.is_finished()
        } else {
            false
        }
    }
}

///
/// # Description
///
/// Converts a raw exit code into an `ExitStatus`.
///
/// # Parameters
///
/// - `raw_code`: Raw exit code returned by the User VM.
///
/// # Returns
///
/// Returns the corresponding `ExitStatus`.
///
fn exit_status_from_exit_code(raw_code: u8) -> ExitStatus {
    let code: i32 = i32::from(raw_code) & 0xff;
    ExitStatus::from_raw(code << 8)
}
