// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::KILL_SIGNAL;
use ::anyhow::Result;
use ::std::{
    ops::ControlFlow::{
        self,
        Break,
        Continue,
    },
    pin::Pin,
};
use ::syslog::{
    debug,
    error,
    info,
    trace,
    warn,
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

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// This structure holds the state of the VM, channels used for orchestrating control commands, and
/// callback functions which implement specific functionality that's different between the MicroVM
/// and Hyperlight.
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
    /// Callback function to load a snapshot.
    load_snapshot: Box<LoadSnapshotFn>,
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
    StartMicroVm,
    LoadSnapshot,
    Pause,
    CreateSnapshot,
    Resume,
    Shutdown,
}

///
/// # Description
///
/// Control plane command responses from the VMM to the I/O thread.
///
#[derive(Debug, PartialEq)]
pub enum IoControlResponse {
    CreateSnapshotMarker,
    MicroVmPaused,
    SnapshotCreated,
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
    CreateSnapshot,
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
        load_snapshot: Box<LoadSnapshotFn>,
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
            load_snapshot,
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
                        let pthread_id: libc::pthread_t = self.vcpu_tid as libc::pthread_t;
                        unsafe { ::libc::pthread_kill(pthread_id, KILL_SIGNAL) };
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
            IoControlCommand::StartMicroVm => {
                if self.state == State::PreBoot {
                    // TODO: separate starting logic from `spawn()` and put it here
                    // This only makes sense when snapshots can already be loaded https://github.com/nanvix/nanvix/issues/948
                    self.state = State::Running;
                    trace!("State: PreBoot -> Running");
                }
                Ok(Continue(()))
            },
            IoControlCommand::LoadSnapshot => {
                if self.state == State::PreBoot {
                    if let Err(e) = (self.load_snapshot)().await {
                        let reason: String =
                            format!("LoadSnapshot: failed to load snapshot: {e:?}");
                        error!("try_receive_from_io_thread(): {reason}");
                        anyhow::bail!(reason);
                    }
                    self.state = State::Paused;
                    trace!("State: PreBoot -> Paused");
                }
                Ok(Continue(()))
            },
            IoControlCommand::Pause => {
                if self.state == State::Running
                    && let Err(e) = self.pause_protocol().await
                {
                    let reason: String = format!("PauseMicroVm: failed to pause microvm: {e:?}");
                    error!("try_receive_from_io_thread(): {reason}");
                    anyhow::bail!(reason);
                }
                Ok(Continue(()))
            },
            IoControlCommand::CreateSnapshot => {
                if self.state == State::Paused
                    && let Err(e) = self.create_snapshot_protocol().await
                {
                    let reason: String =
                        format!("CreateSnapshot: failed to create snapshot: {e:?}");
                    error!("try_receive_from_io_thread(): {reason}");
                    anyhow::bail!(reason);
                }
                Ok(Continue(()))
            },
            IoControlCommand::Resume => {
                if self.state == State::Paused
                    && let Err(error) = self.resume_protocol().await
                {
                    let reason: String =
                        format!("ResumeMicroVm: failed to resume microvm: {error:?}");
                    error!("try_receive_from_io_thread(): {reason}");
                    anyhow::bail!(reason);
                }
                Ok(Continue(()))
            },
            IoControlCommand::Shutdown => {
                debug!("try_receive_from_io_thread(): received shutdown command");

                // After sending an interrupt to the vCPU thread, we continue processing
                // messages until we receive a shutdown message from the vCPU thread itself.
                cfg_if::cfg_if! {
                    // FIXME (#1010): there is currently no way for us to actually interrupt the
                    // hyperlight thread so, instead of waiting for a shutdown message from the vCPU
                    // itself, we kill it here. This may populate the logs with an error message
                    // during shutdown, but is functionally equivalent.
                    if #[cfg(feature = "hyperlight")] {
                        debug!("try_receive_from_io_thread(): killing vcpu thread id: {}", self.vcpu_tid);
                        let pthread_id: libc::pthread_t = self.vcpu_tid as libc::pthread_t;
                        unsafe { ::libc::pthread_kill(pthread_id, KILL_SIGNAL) };
                        self.state = State::ShuttingDown;
                        Ok(Continue(()))
                    } else {
                        // SAFETY: we call pthread_kill on a non-zero TID that we have received from
                        // the VCPU thread after boot, so this is safe.
                        let pthread_id: libc::pthread_t = self.vcpu_tid as libc::pthread_t;
                        debug!("try_receive_from_io_thread(): signaling to vcpu thread (tid={})", self.vcpu_tid);
                        unsafe { ::libc::pthread_kill(pthread_id, crate::vmm::INTERRUPT_SIGNAL) };

                        // Transition to shutting down state.
                        self.state = State::ShuttingDown;
                        Ok(Continue(()))
                    }
                }
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
    /// Attempts to create a snapshot of a paused MicroVM and the channel state.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    async fn create_snapshot_protocol(&mut self) -> Result<()> {
        // At this point, two conditions must be achieved, and they can happen
        // concurrently:
        //
        // A) the vCPU must save the local state; and
        // B) the entire distributed system must pool all outstanding messages in a
        // single spot. This pool is the `channel state` in the snapshot. The channel
        // state must be saved.
        //
        // "B" is already in progress either way, so start "A":
        if let Err(error) = self
            .vcpu_control_tx
            .send(VcpuControlCommand::CreateSnapshot)
            .await
        {
            let reason: String =
                format!("CreateSnapshot: failed to send CreateSnapshot command to vCPU: {error:?}");
            error!("create_snapshot_protocol(): {reason}");
            anyhow::bail!(reason);
        }

        // Now send a round-trip marker to ensure all channels are drained. Send to the
        // I/O thread, but receive from the vCPU. This starts the final step in "B".
        if let Err(error) = self
            .io_control_tx
            .send(IoControlResponse::CreateSnapshotMarker)
            .await
        {
            let reason: String = format!(
                "CreateSnapshot: failed to send CreateSnapshotMarker to I/O thread: {error:?}"
            );
            error!("create_snapshot_protocol(): {reason}");
            anyhow::bail!(reason);
        }

        // Receiving from the vCPU means it has saved the local state and the channel
        // state as files.
        match self.vcpu_control_rx.recv().await {
            Some(VcpuControlResponse::SnapshotCreated) => {
                trace!("Snapshot created");
                if let Err(error) = self
                    .io_control_tx
                    .send(IoControlResponse::SnapshotCreated)
                    .await
                {
                    let reason: String = format!(
                        "CreateSnapshot: failed to send SnapshotCreated response to I/O thread: \
                         {error:?}"
                    );
                    error!("create_snapshot_protocol(): {reason}");
                    anyhow::bail!(reason);
                }
                Ok(())
            },
            Some(VcpuControlResponse::SnapshotCreationFailed) => {
                let reason: String = "vCPU reported snapshot creation failure".to_string();
                error!("create_snapshot_protocol(): {reason}");
                anyhow::bail!(reason);
            },
            Some(other) => {
                let reason: String =
                    format!("unexpected vCPU response during snapshot creation: {other:?}");
                error!("create_snapshot_protocol(): {reason}");
                anyhow::bail!(reason);
            },
            None => {
                let reason: String =
                    "disconnected from the vCPU control response channel".to_string();
                error!("create_snapshot_protocol(): {reason}");
                anyhow::bail!(reason);
            },
        }
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
        h.io_cmd_tx
            .send(IoControlCommand::StartMicroVm)
            .await
            .expect("send StartMicroVm");
        // Request pause.
        h.io_cmd_tx
            .send(IoControlCommand::Pause)
            .await
            .expect("send Pause");

        // Simulate vCPU acknowledging pause.
        let vcpu_resp_tx_clone: mpsc::Sender<VcpuControlResponse> = h.vcpu_resp_tx.clone();
        let io_cmd_tx_clone: mpsc::Sender<IoControlCommand> = h.io_cmd_tx.clone();
        tokio::spawn(async move {
            // Allow orchestrator to enter pause_protocol loop.
            sleep(Duration::from_millis(5)).await;
            let _ = vcpu_resp_tx_clone.send(VcpuControlResponse::Paused).await;
        });

        // Expect MicroVmPaused.
        let r = recv_io_resp(&mut h.io_resp_rx).await;
        assert!(matches!(r, IoControlResponse::MicroVmPaused));
        assert!(h.pause_called.load(Ordering::SeqCst));

        // Resume.
        h.io_cmd_tx
            .send(IoControlCommand::Resume)
            .await
            .expect("send Resume");
        // Expect a Resume command sent to vCPU thread.
        let resume_cmd: VcpuControlCommand =
            timeout(Duration::from_millis(500), h.vcpu_cmd_rx.recv())
                .await?
                .expect("vcpu command channel closed unexpectedly");
        assert!(matches!(resume_cmd, VcpuControlCommand::Resume));
        assert!(h.resume_called.load(Ordering::SeqCst));

        // Now shut everything down.
        h.vcpu_resp_tx
            .send(VcpuControlResponse::Shutdown)
            .await
            .expect("send Shutdown");
        let _mem_cmd = timeout(Duration::from_millis(500), h.mem_cmd_rx.recv()).await?;
        let _shutdown_resp = recv_io_resp(&mut h.io_resp_rx).await; // Shutdown

        let res = handle.await.expect("join failed");
        assert!(res.is_ok());
        Ok(())
    }
}
