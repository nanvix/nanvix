// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    debug,
    error,
    info,
    trace,
    warn,
};
use ::std::{
    ops::ControlFlow::{
        self,
        Break,
        Continue,
    },
    pin::Pin,
};
use ::tokio::{
    select,
    sync::mpsc::{
        Receiver,
        Sender,
    },
    task::JoinHandle,
    time::{
        Duration,
        Instant,
        timeout,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

// This value was chosen so it catches issues without polluting the logs with too many warnings.
pub const TIMEOUT_WARNING_INTERVAL_IN_MS: usize = 10;

/// Timeout for shutdown operations.
/// After this timeout, the orchestrator retries the backend shutdown request.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(5000);

//==================================================================================================
// Types
//==================================================================================================

pub type PauseFn = dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + 'static;
pub type ResumeFn = dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + 'static;
pub type CreateSnapshotFn =
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + 'static;
pub type LoadSnapshotFn =
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + 'static;
pub type ShutdownVcpuFn = dyn Fn(u64) + Send + 'static;

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// This structure holds the state of the VM, channels used for orchestrating control commands, and
/// callback functions which implement specific VMM functionality.
///
pub struct Orchestrator {
    /// The state of the VM.
    state: State,
    /// Thread ID of the vCPU thread.
    vcpu_tid: u64,
    /// Channel that receives commands from the I/O handler.
    io_control_rx: Receiver<IoControlCommand>,
    /// Channel that sends commands to the I/O handler.
    io_control_tx: Sender<IoControlResponse>,
    /// Channel that receives commands from the memory thread.
    _memory_control_rx: Receiver<MemoryControlResponse>,
    /// Channel that sends commands to the memory thread.
    memory_control_tx: Sender<MemoryControlCommand>,
    /// Channel that receives commands from the vCPU thread.
    vcpu_control_rx: Receiver<VcpuControlResponse>,
    /// Channel that sends commands to the vCPU thread.
    vcpu_control_tx: Sender<VcpuControlCommand>,
    /// Callback function to write to the kernel's memory a pause request.
    pause_microvm: Box<PauseFn>,
    /// Callback function to erase a pause request from the kernel's memory.
    resume_microvm: Box<ResumeFn>,
    /// Callback function to create a snapshot.
    _create_snapshot: Box<CreateSnapshotFn>,
    /// Callback function to load a snapshot.
    load_snapshot: Box<LoadSnapshotFn>,
    /// Callback function to request vCPU shutdown (sets shared flag and cancels the vCPU run).
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    shutdown_vcpu: Box<ShutdownVcpuFn>,
    /// Maximum time to wait for the vCPU to confirm shutdown.
    shutdown_timeout: Duration,
}

//==================================================================================================
// Enumerations
//==================================================================================================

///
/// # Description
///
/// States of the VM.
///
#[derive(PartialEq)]
enum State {
    /// Waiting for the VM to start.
    PreBoot,
    /// VM is running.
    Running,
    /// VM is not running.
    Paused,
    /// VM is shutting down, waiting for vCPU to confirm.
    ShuttingDown,
}

///
/// # Description
///
/// Control commands from the I/O handler to the VMM.
///
#[derive(PartialEq)]
pub enum IoControlCommand {
    _StartMicroVm,
    _LoadSnapshotAndRun,
    _PauseMicroVm,
    _CreateSnapshot(String),
    _ResumeMicroVm,
    SystemCallFlushed,
    Shutdown,
}

///
/// # Description
///
/// Control command responses from the VMM to the I/O handler.
///
#[derive(Debug, PartialEq)]
pub enum IoControlResponse {
    MicroVmPaused,
    SnapshotCreated,
    FlushOutput,
    FlushInput,
    Shutdown,
}

///
/// # Description
///
/// Control commands from the VMM to the memory thread.
///
#[derive(PartialEq)]
pub enum MemoryControlCommand {
    Shutdown,
}

///
/// # Description
///
/// Control command responses from the memory thread to the VMM.
///
#[derive(PartialEq)]
pub enum MemoryControlResponse {}

///
/// # Description
///
/// Control commands from the VMM to the vCPU thread.
///
#[derive(PartialEq)]
pub enum VcpuControlCommand {
    CreateSnapshot(String),
    Resume,
}

///
/// # Description
///
/// Control command responses from the vCPU thread to the VMM.
///
#[derive(Debug, PartialEq)]
pub enum VcpuControlResponse {
    Paused,
    Shutdown,
    SnapshotCreated,
    SnapshotCreationFailed,
    Tid(u64),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Orchestrator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vcpu_tid: u64,
        io_control_rx: Receiver<IoControlCommand>,
        io_control_tx: Sender<IoControlResponse>,
        memory_control_rx: Receiver<MemoryControlResponse>,
        memory_control_tx: Sender<MemoryControlCommand>,
        vcpu_control_rx: Receiver<VcpuControlResponse>,
        vcpu_control_tx: Sender<VcpuControlCommand>,
        pause_microvm: Box<PauseFn>,
        resume_microvm: Box<ResumeFn>,
        create_snapshot: Box<CreateSnapshotFn>,
        load_snapshot: Box<LoadSnapshotFn>,
        shutdown_vcpu: Box<ShutdownVcpuFn>,
    ) -> Self {
        Self {
            state: State::PreBoot,
            vcpu_tid,
            io_control_rx,
            io_control_tx,
            _memory_control_rx: memory_control_rx,
            memory_control_tx,
            vcpu_control_rx,
            vcpu_control_tx,
            pause_microvm,
            resume_microvm,
            _create_snapshot: create_snapshot,
            load_snapshot,
            shutdown_vcpu,
            shutdown_timeout: SHUTDOWN_TIMEOUT,
        }
    }

    ///
    /// # Description
    ///
    /// Runs the main orchestrator loop.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    async fn run(&mut self) -> Result<()> {
        loop {
            // If we're in shutting down state, add timeout to prevent indefinite hang.
            if self.state == State::ShuttingDown {
                match timeout(self.shutdown_timeout, self.wait_for_shutdown()).await {
                    Ok(Ok(())) => break,
                    Ok(Err(error)) => {
                        error!("run(): error during shutdown: {error:?}");
                        break;
                    },
                    Err(_) => {
                        error!(
                            "run(): shutdown timeout after {}ms, retrying vCPU shutdown request",
                            self.shutdown_timeout.as_millis()
                        );
                        // Retry the backend-specific shutdown request. A process-directed fatal
                        // signal cannot safely terminate only the vCPU thread.
                        (self.shutdown_vcpu)(self.vcpu_tid);
                        break;
                    },
                }
            }

            select! {
                // Only poll the I/O control channel when I/O is enabled; otherwise skip this branch.
                result = self.io_control_rx.recv() => {
                    match result {
                        Some(command) => {
                            match self.handle_io_control_command(command).await? {
                                Continue(()) => continue,
                                Break(()) => break,
                            }
                        },
                        None => {
                            let reason: String =
                                "disconnected from the input control command channel".to_string();
                            error!("handle_io_control_command(): {reason}");
                            break;
                        },
                    }
                },

                result = self.vcpu_control_rx.recv() => {
                    match result {
                        Some(control_response) => {
                            match self.try_receive_from_vcpu(control_response).await? {
                                Continue(()) => continue,
                                Break(()) => break,

                            }
                        },
                        None => {
                            let reason: String =
                                ("disconnected from the vCPU control response channel").to_string();
                            error!("try_receive_from_vcpu(): {reason}");
                            break
                        },
                    }
                },
            }
        }

        // If we break out of the main loop, we need to shutdown the I/O and
        // the memory thread.
        debug!("run(): exited run loop, cleaning-up");
        self.memory_control_tx
            .send(MemoryControlCommand::Shutdown)
            .await?;

        self.io_control_tx.send(IoControlResponse::Shutdown).await?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Waits for the vCPU thread to send a shutdown message.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    async fn wait_for_shutdown(&mut self) -> Result<()> {
        let mut io_control_open: bool = true;
        loop {
            select! {
                result = self.io_control_rx.recv(), if io_control_open => {
                    // Ignore I/O commands during shutdown.
                    if result.is_none() {
                        let reason: String =
                            "I/O control channel closed during shutdown".to_string();
                        warn!("wait_for_shutdown(): {reason}");
                        io_control_open = false;
                    }
                },

                result = self.vcpu_control_rx.recv() => {
                    match result {
                        Some(VcpuControlResponse::Shutdown) => {
                            info!("wait_for_shutdown(): vCPU shutdown confirmed");
                            return Ok(());
                        },
                        Some(other) => {
                            warn!("wait_for_shutdown(): unexpected vCPU response: {other:?}");
                        },
                        None => {
                            let reason: String =
                                "vCPU control channel closed during shutdown".to_string();
                            error!("wait_for_shutdown(): {reason}");
                            return Err(anyhow::anyhow!(reason));
                        },
                    }
                },
            }
        }
    }

    ///
    /// # Description
    ///
    /// Spawns the orchestrator's main run loop on a background Tokio task, returning a handle
    /// to it. This consumes the orchestrator instance so that it can live for the entire
    /// lifetime of the asynchronous task (required for `tokio::spawn` which needs a `'static`
    /// future). The caller may `.await` the returned handle to obtain the `Result<()>` produced
    /// by `run()` or detach it by ignoring the handle.
    ///
    /// # Returns
    ///
    /// A [`JoinHandle`] to the spawned task which yields a `Result<()>` once the orchestrator's
    /// event loop terminates.
    ///
    /// # Usage
    ///
    /// ```ignore
    /// let handle = orchestrator.spawn();
    /// // ... perform other async work ...
    /// let result = handle.await?; // Propagate any error from the run loop.
    /// ```
    ///
    pub fn spawn(self) -> JoinHandle<Result<()>> {
        trace!("spawn()");
        ::tokio::spawn(async move {
            let mut orchestrator: Orchestrator = self;
            orchestrator.run().await
        })
    }

    async fn handle_io_control_command(
        &mut self,
        command: IoControlCommand,
    ) -> Result<ControlFlow<()>> {
        match command {
            IoControlCommand::_StartMicroVm => {
                if self.state == State::PreBoot {
                    // TODO: separate starting logic from `spawn()` and put it here
                    // This only makes sense when snapshots can already be loaded https://github.com/nanvix/nanvix/issues/948
                    self.state = State::Running;
                    trace!("State: PreBoot -> Running");
                }
                Ok(Continue(()))
            },
            IoControlCommand::_LoadSnapshotAndRun => {
                if self.state == State::PreBoot {
                    if let Err(e) = (self.load_snapshot)().await {
                        let reason: String =
                            format!("LoadSnapshotAndRun: failed to load snapshot: {e:?}");
                        error!("handle_io_control_command(): {reason}");
                        anyhow::bail!(reason);
                    }
                    trace!("State: PreBoot -> Paused");

                    // The I/O handler sends messages to pre-boot VMs by default, so it does not
                    // need an explicit resume notification.

                    if let Err(e) = self.resume_protocol().await {
                        let reason: String =
                            format!("LoadSnapshotAndRun: failed to resume microvm: {e:?}");
                        error!("handle_io_control_command(): {reason}");
                        anyhow::bail!(reason);
                    }
                }
                Ok(Continue(()))
            },
            IoControlCommand::_PauseMicroVm => {
                if self.state == State::Running
                    && let Err(e) = self.pause_protocol().await
                {
                    let reason: String = format!("PauseMicroVm: failed to pause microvm: {e:?}");
                    error!("handle_io_control_command(): {reason}");
                    anyhow::bail!(reason);
                }
                Ok(Continue(()))
            },
            IoControlCommand::_CreateSnapshot(filepath) => {
                if self.state == State::Paused {
                    if let Err(error) = self
                        .vcpu_control_tx
                        .send(VcpuControlCommand::CreateSnapshot(filepath))
                        .await
                    {
                        let reason: String = format!(
                            "CreateSnapshot: failed to send CreateSnapshot command to vCPU: \
                             {error:?}"
                        );
                        error!("handle_io_control_command(): {reason}");
                        anyhow::bail!(reason);
                    }

                    match self.vcpu_control_rx.recv().await {
                        Some(VcpuControlResponse::SnapshotCreated) => {
                            trace!("Snapshot created");
                        },
                        Some(VcpuControlResponse::SnapshotCreationFailed) => {
                            let reason: String =
                                "vCPU reported snapshot creation failure".to_string();
                            error!("handle_io_control_command(): {reason}");
                            anyhow::bail!(reason);
                        },
                        Some(other) => {
                            let reason: String = format!(
                                "unexpected vCPU response during snapshot creation: {other:?}"
                            );
                            error!("handle_io_control_command(): {reason}");
                            anyhow::bail!(reason);
                        },
                        None => {
                            let reason: String =
                                "disconnected from the vCPU control response channel".to_string();
                            error!("handle_io_control_command(): {reason}");
                            anyhow::bail!(reason);
                        },
                    }
                }
                Ok(Continue(()))
            },
            IoControlCommand::_ResumeMicroVm => {
                if self.state == State::Paused {
                    // TODO (#945): Tell the I/O handler that it may resume sending messages.
                    if let Err(error) = self.resume_protocol().await {
                        let reason: String =
                            format!("ResumeMicroVm: failed to resume microvm: {error:?}");
                        error!("handle_io_control_command(): {reason}");
                        anyhow::bail!(reason);
                    }
                }
                Ok(Continue(()))
            },
            IoControlCommand::SystemCallFlushed => {
                // NOTE: this will be unreachable once the communication is fully implemented
                // `SystemCallFlushed` should only be sent in the middle of `pause_protocol`.
                // In fact, it should already be unreachable, but it cannot be tested ATM.
                Ok(Continue(()))
            },
            IoControlCommand::Shutdown => {
                debug!("handle_io_control_command(): received shutdown command");

                // After requesting vCPU shutdown, continue processing messages until the vCPU
                // thread confirms that it stopped.
                debug!(
                    "handle_io_control_command(): signaling to vcpu thread (tid={})",
                    self.vcpu_tid
                );
                (self.shutdown_vcpu)(self.vcpu_tid);

                // Transition to shutting down state.
                self.state = State::ShuttingDown;
                Ok(Continue(()))
            },
        }
    }

    async fn try_receive_from_vcpu(
        &mut self,
        control_response: VcpuControlResponse,
    ) -> Result<ControlFlow<()>> {
        match control_response {
            VcpuControlResponse::Paused => {
                let reason: String =
                    "paused command should only be received during the pause protocol".to_string();
                error!("{reason}");
                Err(anyhow::anyhow!(reason))
            },

            VcpuControlResponse::Shutdown => {
                info!("try_receive_from_vcpu(): vCPU shutdown");
                Ok(Break(()))
            },
            VcpuControlResponse::SnapshotCreated => {
                let reason: String =
                    "SnapshotCreated should only be received during snapshot creation".to_string();
                error!("{reason}");
                Err(anyhow::anyhow!(reason))
            },
            VcpuControlResponse::SnapshotCreationFailed => {
                let reason: String = "SnapshotCreationFailed should only be received during \
                                      snapshot creation"
                    .to_string();
                error!("{reason}");
                Err(anyhow::anyhow!(reason))
            },
            VcpuControlResponse::Tid(_) => {
                let reason: String = "The tid is only sent when the vCPU thread is spawned. \
                                      Sending it in the middle of a pause protocol is against the \
                                      protocol"
                    .to_string();
                error!("{reason}");
                Err(anyhow::anyhow!(reason))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to pause the MicroVM and its I/O communication.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    async fn pause_protocol(&mut self) -> Result<()> {
        // TODO (#945): Tell the I/O handler to flush before pausing.
        (self.pause_microvm)().await?;
        // Wait for the MicroVM to confirm it has paused without busy spinning.
        let start: Instant = Instant::now();
        let warn_interval: Duration = Duration::from_millis(TIMEOUT_WARNING_INTERVAL_IN_MS as u64);
        loop {
            match timeout(warn_interval, self.vcpu_control_rx.recv()).await {
                Ok(Some(command)) => {
                    if command == VcpuControlResponse::Paused {
                        break;
                    } else {
                        let reason: String = "during the pause protocol we only expect a `Paused` \
                                              command from the vCPU"
                            .to_string();
                        error!("pause_protocol(): {reason}");
                        anyhow::bail!(reason)
                    }
                },
                Ok(None) => {
                    let reason: String = "the vcpu control channel closed".to_string();
                    error!("pause_protocol(): {reason}");
                    anyhow::bail!(reason)
                },
                Err(_error) => {
                    // Timeout elapsed; log progressively.
                    let elapsed_ms: usize = start.elapsed().as_millis() as usize;
                    warn!(
                        "pause_protocol(): {}ms have passed waiting for `Paused` message from vCPU",
                        elapsed_ms
                    );
                    continue;
                },
            }
        }
        trace!("MicroVM paused");
        // Flush pending output.
        self.io_control_tx
            .send(IoControlResponse::FlushOutput)
            .await?;
        // TODO (#945): Stop input delivery while paused and wait for acknowledgement.
        self.io_control_tx
            .send(IoControlResponse::FlushInput)
            .await?;
        self.receive_system_call_flushed().await?;
        self.state = State::Paused;
        trace!("State: Running -> Paused");
        self.io_control_tx
            .send(IoControlResponse::MicroVmPaused)
            .await?;
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a `SystemCallFlushed` message from the control input.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    async fn receive_system_call_flushed(&mut self) -> Result<()> {
        let start: Instant = Instant::now();
        let warn_interval: Duration = Duration::from_millis(TIMEOUT_WARNING_INTERVAL_IN_MS as u64);
        loop {
            match timeout(warn_interval, self.io_control_rx.recv()).await {
                Ok(Some(IoControlCommand::SystemCallFlushed)) => break,
                Ok(Some(_other)) => {
                    // Ignore unrelated messages during flushing.
                    continue;
                },
                Ok(None) => {
                    let reason: String =
                        "io_control_rx closed while waiting for SystemCallFlushed".to_string();
                    error!("receive_system_call_flushed(): {reason}");
                    anyhow::bail!(reason)
                },
                Err(_) => {
                    let elapsed_ms: usize = start.elapsed().as_millis() as usize;
                    warn!("{}ms have passed waiting for `SystemCallFlushed`", elapsed_ms);
                    continue;
                },
            }
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to resume execution of a paused MicroVM.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    async fn resume_protocol(&mut self) -> Result<()> {
        // Write to the kernel a pause is no longer requested.
        (self.resume_microvm)().await?;
        // Tell microvm to resume
        self.vcpu_control_tx
            .send(VcpuControlCommand::Resume)
            .await?;
        self.state = State::Running;
        trace!("State: Paused -> Running");
        Ok(())
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
