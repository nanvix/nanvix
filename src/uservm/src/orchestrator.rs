// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
use crate::vmm::KILL_SIGNAL;
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
/// After this timeout, the orchestrator will forcefully terminate the vCPU thread.
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
pub type ShutdownVcpuFn = dyn Fn() + Send + 'static;

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
    /// Channel that receives commands from the I/O thread.
    io_control_rx: Receiver<IoControlCommand>,
    /// Channel that sends commands to the I/O thread.
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
/// Control plane commands from the I/O thread to the VMM.
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
/// Control plane command responses from the VMM to the I/O thread.
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
/// Control plane commands from the VMM to the memory thread.
///
#[derive(PartialEq)]
pub enum MemoryControlCommand {
    Shutdown,
}

///
/// # Description
///
/// Control plane command responses from the memory thread to the VMM.
///
#[derive(PartialEq)]
pub enum MemoryControlResponse {}

///
/// # Description
///
/// Control plane commands from the VMM to the vCPU thread.
///
#[derive(PartialEq)]
pub enum VcpuControlCommand {
    CreateSnapshot(String),
    Resume,
}

///
/// # Description
///
/// Control plane command responses from the vCPU thread to the VMM.
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
                match timeout(SHUTDOWN_TIMEOUT, self.wait_for_shutdown()).await {
                    Ok(Ok(())) => break,
                    Ok(Err(error)) => {
                        error!("run(): error during shutdown: {error:?}");
                        break;
                    },
                    Err(_) => {
                        error!(
                            "run(): shutdown timeout after {}ms, forcefully terminating vCPU \
                             thread",
                            SHUTDOWN_TIMEOUT.as_millis()
                        );
                        // Forcefully terminate the vCPU thread.
                        #[cfg(target_os = "linux")]
                        {
                            let pthread_id: libc::pthread_t = self.vcpu_tid as libc::pthread_t;
                            unsafe { ::libc::pthread_kill(pthread_id, KILL_SIGNAL) };
                        }
                        #[cfg(target_os = "windows")]
                        {
                            (self.shutdown_vcpu)();
                        }
                        break;
                    },
                }
            }

            select! {
                // Only poll the I/O control channel when I/O is enabled; otherwise skip this branch.
                result = self.io_control_rx.recv() => {
                    match result {
                        Some(command) => {
                            match self.try_receive_from_io_thread(command).await? {
                                Continue(()) => continue,
                                Break(()) => break,
                            }
                        },
                        None => {
                            let reason: String =
                                "disconnected from the input control command channel".to_string();
                            error!("try_receive_from_io_thread(): {reason}");
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
        loop {
            select! {
                result = self.io_control_rx.recv() => {
                    // Ignore I/O commands during shutdown.
                    if result.is_none() {
                        let reason: String =
                            "I/O control channel closed during shutdown".to_string();
                        warn!("wait_for_shutdown(): {reason}");
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

    async fn try_receive_from_io_thread(
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
                        error!("handle_command(): {reason}");
                        anyhow::bail!(reason);
                    }
                    trace!("State: PreBoot -> Paused");

                    // The Linux daemon should send messages to PreBoot VMMs by default,
                    // so there's no need to tell it to resume sending messages.

                    if let Err(e) = self.resume_protocol().await {
                        let reason: String =
                            format!("LoadSnapshotAndRun: failed to resume microvm: {e:?}");
                        error!("try_receive_from_io_thread(): {reason}");
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
                    error!("try_receive_from_io_thread(): {reason}");
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
                        error!("try_receive_from_io_thread(): {reason}");
                        anyhow::bail!(reason);
                    }

                    match self.vcpu_control_rx.recv().await {
                        Some(VcpuControlResponse::SnapshotCreated) => {
                            trace!("Snapshot created");
                        },
                        Some(VcpuControlResponse::SnapshotCreationFailed) => {
                            let reason: String =
                                "vCPU reported snapshot creation failure".to_string();
                            error!("try_receive_from_io_thread(): {reason}");
                            anyhow::bail!(reason);
                        },
                        Some(other) => {
                            let reason: String = format!(
                                "unexpected vCPU response during snapshot creation: {other:?}"
                            );
                            error!("try_receive_from_io_thread(): {reason}");
                            anyhow::bail!(reason);
                        },
                        None => {
                            let reason: String =
                                "disconnected from the vCPU control response channel".to_string();
                            error!("try_receive_from_io_thread(): {reason}");
                            anyhow::bail!(reason);
                        },
                    }
                }
                Ok(Continue(()))
            },
            IoControlCommand::_ResumeMicroVm => {
                if self.state == State::Paused {
                    // TODO: tell linuxd it's fine to send more messages https://github.com/nanvix/nanvix/issues/945
                    if let Err(error) = self.resume_protocol().await {
                        let reason: String =
                            format!("ResumeMicroVm: failed to resume microvm: {error:?}");
                        error!("try_receive_from_io_thread(): {reason}");
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
                debug!("try_receive_from_io_thread(): received shutdown command");

                // After sending an interrupt to the vCPU thread, we continue processing
                // messages until we receive a shutdown message from the vCPU thread itself.
                // SAFETY: we call pthread_kill on a non-zero TID that we have received from
                // the VCPU thread after boot, so this is safe.
                debug!(
                    "try_receive_from_io_thread(): signaling to vcpu thread (tid={})",
                    self.vcpu_tid
                );
                #[cfg(target_os = "linux")]
                {
                    let pthread_id: libc::pthread_t = self.vcpu_tid as libc::pthread_t;
                    unsafe { ::libc::pthread_kill(pthread_id, crate::vmm::INTERRUPT_SIGNAL) };
                }
                #[cfg(target_os = "windows")]
                {
                    debug!(
                        "try_receive_from_io_thread(): requesting vCPU shutdown (tid={})",
                        self.vcpu_tid
                    );
                    (self.shutdown_vcpu)();
                }

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
    /// Attempts to pause the execution of the MicroVM and the communication with the Linux daemon.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    async fn pause_protocol(&mut self) -> Result<()> {
        // TODO: tell linuxd to flush (Running -> Flushing) https://github.com/nanvix/nanvix/issues/945
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
        // Flush output to linuxd
        self.io_control_tx
            .send(IoControlResponse::FlushOutput)
            .await?;
        // TODO: tell linuxd to stop sending messages (Flushing -> Paused) https://github.com/nanvix/nanvix/issues/945
        // TODO: get a response from linuxd https://github.com/nanvix/nanvix/issues/945
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
mod tests {
    use super::*;
    use ::anyhow::Result as AnyResult;
    use ::std::{
        sync::{
            Arc,
            atomic::{
                AtomicBool,
                Ordering,
            },
        },
        time::Duration,
    };
    use ::tokio::{
        sync::mpsc,
        time::{
            sleep,
            timeout,
        },
    };

    // Helper to build a fresh orchestrator and all channels for each test.
    struct Harness {
        orchestrator: Orchestrator,
        // Channels we observe.
        io_cmd_tx: mpsc::Sender<IoControlCommand>,
        io_resp_rx: mpsc::Receiver<IoControlResponse>,
        mem_cmd_rx: mpsc::Receiver<MemoryControlCommand>,
        vcpu_resp_tx: mpsc::Sender<VcpuControlResponse>,
        vcpu_cmd_rx: mpsc::Receiver<VcpuControlCommand>,
        pause_called: Arc<AtomicBool>,
        resume_called: Arc<AtomicBool>,
    }

    fn build_harness() -> Harness {
        let (io_cmd_tx, io_cmd_rx) = mpsc::channel(8);
        let (io_resp_tx, io_resp_rx) = mpsc::channel(8);
        let (mem_resp_tx, mem_resp_rx) = mpsc::channel(1); // Memory -> VMM (unused)
        let _ = mem_resp_tx; // silence unused warning
        let (mem_cmd_tx, mem_cmd_rx) = mpsc::channel(2); // VMM -> Memory (we assert on this)
        let (vcpu_resp_tx, vcpu_resp_rx) = mpsc::channel(8); // vCPU -> VMM
        let (vcpu_cmd_tx, vcpu_cmd_rx) = mpsc::channel(8); // VMM -> vCPU

        let pause_called: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let resume_called: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let snapshot_called: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        let pause_flag: Arc<AtomicBool> = pause_called.clone();
        let resume_flag: Arc<AtomicBool> = resume_called.clone();
        let snapshot_flag: Arc<AtomicBool> = snapshot_called.clone();

        let orchestrator: Orchestrator = Orchestrator::new(
            0, // vcpu_tid (unused for these tests)
            io_cmd_rx,
            io_resp_tx,
            mem_resp_rx,
            mem_cmd_tx,
            vcpu_resp_rx,
            vcpu_cmd_tx,
            Box::new(move || {
                let flag = pause_flag.clone();
                Box::pin(async move {
                    flag.store(true, Ordering::SeqCst);
                    Ok(())
                })
            }),
            Box::new(move || {
                let flag = resume_flag.clone();
                Box::pin(async move {
                    flag.store(true, Ordering::SeqCst);
                    Ok(())
                })
            }),
            Box::new(move || {
                let flag = snapshot_flag.clone();
                Box::pin(async move {
                    flag.store(true, Ordering::SeqCst);
                    Ok(())
                })
            }),
            {
                let snapshot_flag2: Arc<AtomicBool> = snapshot_called.clone();
                Box::new(move || {
                    let flag = snapshot_flag2.clone();
                    Box::pin(async move {
                        flag.store(true, Ordering::SeqCst);
                        Ok(())
                    })
                })
            },
            Box::new(|| {}),
        );

        Harness {
            orchestrator,
            io_cmd_tx,
            io_resp_rx,
            mem_cmd_rx,
            vcpu_resp_tx,
            vcpu_cmd_rx,
            pause_called,
            resume_called,
        }
    }

    // Utility to receive the next IO response with a timeout.
    async fn recv_io_resp(rx: &mut mpsc::Receiver<IoControlResponse>) -> IoControlResponse {
        timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("io response timed out")
            .expect("io response channel closed")
    }

    #[tokio::test]
    async fn run_shutdown_sends_cleanup_messages() -> AnyResult<()> {
        let mut h: Harness = build_harness();
        let handle: JoinHandle<Result<()>> = h.orchestrator.spawn();

        // Trigger shutdown via vCPU.
        h.vcpu_resp_tx.send(VcpuControlResponse::Shutdown).await?;

        // Expect memory shutdown command after run loop exits.
        let mem_cmd: MemoryControlCommand =
            timeout(Duration::from_millis(500), h.mem_cmd_rx.recv())
                .await?
                .expect("memory control channel closed unexpectedly");
        assert!(matches!(mem_cmd, MemoryControlCommand::Shutdown));

        // Expect IO shutdown response.
        let io_resp: IoControlResponse = recv_io_resp(&mut h.io_resp_rx).await;
        assert!(matches!(io_resp, IoControlResponse::Shutdown));

        // Join orchestrator task (should be Ok(()))
        let res: Result<()> = handle.await.expect("orchestrator join failed");
        assert!(res.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn pause_and_resume_flow() -> AnyResult<()> {
        let mut h: Harness = build_harness();
        let handle: JoinHandle<Result<()>> = h.orchestrator.spawn();

        // Move to Running state.
        h.io_cmd_tx.send(IoControlCommand::_StartMicroVm).await?;
        // Request pause.
        h.io_cmd_tx.send(IoControlCommand::_PauseMicroVm).await?;

        // Simulate vCPU acknowledging pause and linux daemon flush.
        let vcpu_resp_tx_clone: mpsc::Sender<VcpuControlResponse> = h.vcpu_resp_tx.clone();
        let io_cmd_tx_clone: mpsc::Sender<IoControlCommand> = h.io_cmd_tx.clone();
        tokio::spawn(async move {
            // Allow orchestrator to enter pause_protocol loop.
            sleep(Duration::from_millis(5)).await;
            let _ = vcpu_resp_tx_clone.send(VcpuControlResponse::Paused).await;
            // Allow orchestrator to send FlushOutput & FlushInput, then provide SystemCallFlushed.
            sleep(Duration::from_millis(5)).await;
            let _ = io_cmd_tx_clone
                .send(IoControlCommand::SystemCallFlushed)
                .await;
        });

        // Expect FlushOutput, FlushInput, MicroVmPaused in order.
        let r1 = recv_io_resp(&mut h.io_resp_rx).await;
        assert!(matches!(r1, IoControlResponse::FlushOutput));
        let r2 = recv_io_resp(&mut h.io_resp_rx).await;
        assert!(matches!(r2, IoControlResponse::FlushInput));
        let r3 = recv_io_resp(&mut h.io_resp_rx).await;
        assert!(matches!(r3, IoControlResponse::MicroVmPaused));
        assert!(h.pause_called.load(Ordering::SeqCst));

        // Resume.
        h.io_cmd_tx.send(IoControlCommand::_ResumeMicroVm).await?;
        // Expect a Resume command sent to vCPU thread.
        let resume_cmd: VcpuControlCommand =
            timeout(Duration::from_millis(500), h.vcpu_cmd_rx.recv())
                .await?
                .expect("vcpu command channel closed unexpectedly");
        assert!(matches!(resume_cmd, VcpuControlCommand::Resume));
        assert!(h.resume_called.load(Ordering::SeqCst));

        // Now shut everything down.
        h.vcpu_resp_tx.send(VcpuControlResponse::Shutdown).await?;
        let _mem_cmd = timeout(Duration::from_millis(500), h.mem_cmd_rx.recv()).await?;
        let _shutdown_resp = recv_io_resp(&mut h.io_resp_rx).await; // Shutdown

        let res = handle.await.expect("join failed");
        assert!(res.is_ok());
        Ok(())
    }
}
