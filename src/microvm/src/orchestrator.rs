// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "hyperlight"))]
use crate::vmm::INTERRUPT_SIGNAL;
use ::anyhow::Result;
#[cfg(not(feature = "hyperlight"))]
use ::libc::{
    pthread_kill,
    pthread_t,
};
use ::mio::{
    Events,
    Poll,
    Token,
    Waker,
};
use ::std::{
    ops::ControlFlow::{
        self,
        Break,
        Continue,
    },
    sync::{
        Arc,
        mpsc::{
            Receiver,
            Sender,
            TryRecvError,
        },
    },
    time::Instant,
};
use ::syslog::{
    debug,
    error,
    info,
    trace,
    warn,
};

//==================================================================================================
// Constants
//==================================================================================================

// This value was chosen so it catches issues without polluting the logs with too many warnings.
pub const TIMEOUT_WARNING_INTERVAL_IN_MS: usize = 10;

/// Token represnting an event notification from inbound queues from the VM.
pub const WAKER_TOKEN: Token = Token(0);

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// Auxiliary enum to disambiguate when we are breaking out of a function because there are no more
/// messages to read, or because we need to shutdown.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BreakReason {
    Empty,
    Shutdown,
}

///
/// # Description
///
/// This structure holds the state of the VM, channels used for orchestrating control commands, and
/// callback functions which implement specific functionality that's different between the MicroVM
/// and Hyperlight.
///
pub struct Orchestrator {
    // The state of the VM.
    state: State,
    /// Poll structure to monitor incoming queues.
    poll: Poll,
    // FIXME (#1009): once the I/O channels are optional, we will be able to infer if io_enabled
    // from whether the channels are None or not.
    /// Whether the I/O thread is enabled or not.
    io_enabled: bool,
    /// Thread ID of the vCPU thread.
    #[cfg(not(feature = "hyperlight"))]
    vcpu_tid: u64,
    /// Waker token for the I/O thread.
    io_thread_waker: Option<Arc<Waker>>,
    // Channel that receives commands from the I/O thread.
    io_control_rx: Receiver<IoControlCommand>,
    // Channel that sends commands to the I/O thread.
    io_control_tx: Sender<IoControlResponse>,
    /// Waker token for the memory thread.
    memory_thread_waker: Arc<Waker>,
    // Channel that receives commands from the memory thread.
    _memory_control_rx: Receiver<MemoryControlResponse>,
    // Channel that sends commands to the memory thread.
    memory_control_tx: Sender<MemoryControlCommand>,
    // Channel that receives commands from the vCPU thread.
    vcpu_control_rx: Receiver<VcpuControlResponse>,
    // Channel that sends commands to the vCPU thread.
    vcpu_control_tx: Sender<VcpuControlCommand>,
    // Callback function to write to the kernel's memory a pause request.
    pause_microvm: Box<dyn Fn() -> Result<()> + Send + 'static>,
    // Callback function to erase a pause request from the kernel's memory.
    resume_microvm: Box<dyn Fn() -> Result<()> + Send + 'static>,
    // Callback function to create a snapshot.
    create_snapshot: Box<dyn Fn() -> Result<()> + Send + 'static>,
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
    _PauseAndCreateSnapshot,
    _CreateSnapshot,
    _ResumeMicroVm,
    LinuxDaemonFlushed,
    Shutdown,
}

///
/// # Description
///
/// Control plane command responses from the VMM to the I/O thread.
///
#[derive(PartialEq)]
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
    #[cfg_attr(feature = "hyperlight", allow(dead_code))]
    Shutdown,
    #[cfg_attr(feature = "hyperlight", allow(dead_code))]
    Tid(u64),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Orchestrator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        poll: Poll,
        io_enabled: bool,
        #[cfg(not(feature = "hyperlight"))] vcpu_tid: u64,
        io_thread_waker: Option<Arc<Waker>>,
        io_control_rx: Receiver<IoControlCommand>,
        io_control_tx: Sender<IoControlResponse>,
        memory_thread_waker: Arc<Waker>,
        memory_control_rx: Receiver<MemoryControlResponse>,
        memory_control_tx: Sender<MemoryControlCommand>,
        vcpu_control_rx: Receiver<VcpuControlResponse>,
        vcpu_control_tx: Sender<VcpuControlCommand>,
        pause_microvm: Box<dyn Fn() -> Result<()> + Send + 'static>,
        resume_microvm: Box<dyn Fn() -> Result<()> + Send + 'static>,
        create_snapshot: Box<dyn Fn() -> Result<()> + Send + 'static>,
    ) -> Self {
        Self {
            state: State::PreBoot,
            poll,
            io_enabled,
            #[cfg(not(feature = "hyperlight"))]
            vcpu_tid,
            io_thread_waker,
            io_control_rx,
            io_control_tx,
            memory_thread_waker,
            _memory_control_rx: memory_control_rx,
            memory_control_tx,
            vcpu_control_rx,
            vcpu_control_tx,
            pause_microvm,
            resume_microvm,
            create_snapshot,
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
    pub fn run(&mut self) -> Result<()> {
        let mut events: Events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);

        'main_loop: loop {
            self.poll.poll(&mut events, None)?;

            // We must drain each socket/queue until they return WouldBlock in order to not miss
            // any messages. We surface a WouldBlock or a queue being empty with a Break().
            if self.io_enabled {
                'io_recv_loop: loop {
                    match self.try_receive_from_io_thread()? {
                        Continue(()) => {},
                        Break(BreakReason::Empty) => break 'io_recv_loop,
                        Break(BreakReason::Shutdown) => break 'main_loop,
                    }
                }
            }
            'vcpu_recv_loop: loop {
                match self.try_receive_from_vcpu()? {
                    Continue(()) => {},
                    Break(BreakReason::Empty) => break 'vcpu_recv_loop,
                    Break(BreakReason::Shutdown) => break 'main_loop,
                }
            }
        }

        // If we break out of the main loop, we need to shutdown the I/O and
        // the memory thread.
        debug!("run(): exited run loop, cleaning-up");
        self.memory_control_tx
            .send(MemoryControlCommand::Shutdown)?;
        self.memory_thread_waker.wake()?;

        self.io_control_tx.send(IoControlResponse::Shutdown)?;
        if let Some(io_thread_waker) = &self.io_thread_waker {
            io_thread_waker.wake()?;
        }

        Ok(())
    }

    fn try_receive_from_io_thread(&mut self) -> Result<ControlFlow<BreakReason, ()>> {
        match self.io_control_rx.try_recv() {
            Ok(command) => match command {
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
                        // TODO: load snapshot https://github.com/nanvix/nanvix/issues/948
                        trace!("State: PreBoot -> Paused");

                        // The Linux daemon should send messages to PreBoot VMMs by default,
                        // so there's no need to tell it to resume sending messages.

                        if let Err(e) = self.resume_protocol() {
                            let reason: String =
                                format!("LoadSnapshotAndRun: failed to resume microvm: {e:?}");
                            error!("try_receive_from_io_thread(): {reason}");
                            anyhow::bail!(reason);
                        }
                    }
                    Ok(Continue(()))
                },
                IoControlCommand::_PauseMicroVm => {
                    if self.state == State::Running {
                        if let Err(e) = self.pause_protocol() {
                            let reason: String =
                                format!("PauseMicroVm: failed to pause microvm: {e:?}");
                            error!("try_receive_from_io_thread(): {reason}");
                            anyhow::bail!(reason);
                        }
                    }
                    Ok(Continue(()))
                },
                IoControlCommand::_PauseAndCreateSnapshot => {
                    if self.state == State::Running {
                        if let Err(e) = self.pause_protocol() {
                            let reason: String =
                                format!("PauseAndCreateSnapshot: failed to pause microvm: {e:?}");
                            error!("try_receive_from_io_thread(): {reason}");
                            anyhow::bail!(reason);
                        }
                        if let Err(e) = (self.create_snapshot)() {
                            let reason: String =
                                format!("PauseAndCreateSnapshot: failed to create snapshot: {e:?}");
                            error!("try_receive_from_io_thread(): {reason}");
                            anyhow::bail!(reason);
                        }
                        trace!("Snapshot created");
                        self.io_control_tx
                            .send(IoControlResponse::SnapshotCreated)?;
                    }
                    Ok(Continue(()))
                },
                IoControlCommand::_CreateSnapshot => {
                    if self.state == State::Paused {
                        if let Err(e) = (self.create_snapshot)() {
                            let reason: String =
                                format!("CreateSnapshot: failed to create snapshot: {e:?}");
                            error!("try_receive_from_io_thread(): {reason}");
                            anyhow::bail!(reason);
                        }
                        trace!("Snapshot created");
                        self.io_control_tx
                            .send(IoControlResponse::SnapshotCreated)?;
                    }
                    Ok(Continue(()))
                },
                IoControlCommand::_ResumeMicroVm => {
                    if self.state == State::Paused {
                        // TODO: tell linuxd it's fine to send more messages https://github.com/nanvix/nanvix/issues/945
                        if let Err(e) = self.resume_protocol() {
                            let reason: String =
                                format!("ResumeMicroVm: failed to resume microvm: {e:?}");
                            error!("try_receive_from_io_thread(): {reason}");
                            anyhow::bail!(reason);
                        }
                    }
                    Ok(Continue(()))
                },
                IoControlCommand::LinuxDaemonFlushed => {
                    // NOTE: this will be unreachable once the communication is fully implemented
                    // `LinuxDaemonFlushed` should only be sent in the middle of `pause_protocol`.
                    // In fact, it should already be unreachable, but it cannot be tested ATM.
                    Ok(Continue(()))
                },
                IoControlCommand::Shutdown => {
                    debug!("try_receive_from_io_thread(): received shutdown command");

                    // After sending an interrupt to the vCPU thread, we continue processing
                    // messages until we receive a shutdown message from the vCPU thread itself.
                    cfg_if::cfg_if! {
                        // FIXME (#1010): there is currently no way for us to actually interrupt
                        // the hyperlight thread so, instead of waiting for a shutdown message from
                        // the vCPU itself, we exit here. This may populate the logs with an error
                        // message during shutdown, but is functionally equivalent.
                        if #[cfg(feature = "hyperlight")] {
                            Ok(Break(BreakReason::Shutdown))
                        } else {
                            // SAFETY: we call pthread_kill on a non-zero TID that we have received from
                            // the VCPU thread after boot, so this is safe.
                            let pthread_id: pthread_t = self.vcpu_tid as pthread_t;
                            unsafe { pthread_kill(pthread_id, INTERRUPT_SIGNAL) };

                            Ok(Continue(()))
                        }
                    }
                },
            },
            Err(TryRecvError::Empty) => Ok(Break(BreakReason::Empty)),
            Err(TryRecvError::Disconnected) => {
                let reason: String =
                    ("disconnected from the input control command channel").to_string();
                error!("try_receive_from_io_thread(): {reason}");
                Ok(Break(BreakReason::Shutdown))
            },
        }
    }

    fn try_receive_from_vcpu(&mut self) -> Result<ControlFlow<BreakReason, ()>> {
        match self.vcpu_control_rx.try_recv() {
            Ok(VcpuControlResponse::Paused) => {
                let reason: String =
                    "paused command should only be received during the pause protocol".to_string();
                error!("{reason}");
                Err(anyhow::anyhow!(reason))
            },
            Ok(VcpuControlResponse::Shutdown) => {
                info!("try_receive_from_vcpu(): vCPU shutdown");
                Ok(Break(BreakReason::Shutdown))
            },
            Ok(VcpuControlResponse::Tid(_)) => {
                let reason: String = "The tid is only sent when the vCPU thread is spawned. \
                                      Sending it in the middle of a pause protocol is against the \
                                      protocol"
                    .to_string();
                error!("{reason}");
                Err(anyhow::anyhow!(reason))
            },
            Err(TryRecvError::Empty) => Ok(Break(BreakReason::Empty)),
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the vmm has disconnected".to_string();
                error!("try_receive_from_io_thread(): {reason}");
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
    fn pause_protocol(&mut self) -> Result<()> {
        // TODO: tell linuxd to flush (Running -> Flushing) https://github.com/nanvix/nanvix/issues/945
        (self.pause_microvm)()?;
        // Wait for the MicroVM to confirm it has paused.
        let start: Instant = Instant::now();
        let mut counter: usize = 1;
        loop {
            match self.vcpu_control_rx.try_recv() {
                Ok(command) => {
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
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => {
                    let reason: String = "the vmm has disconnected".to_string();
                    error!("pause_protocol(): {reason}");
                    anyhow::bail!(reason)
                },
            }
            // Log a warning and increment the counter every TIMEOUT_WARNING_INTERVAL_IN_MS ms.
            let elapsed_time: usize = start.elapsed().as_millis() as usize;
            if elapsed_time > TIMEOUT_WARNING_INTERVAL_IN_MS * counter {
                warn!(
                    "pause_protocol(): {}ms have passed waiting for `Paused` message from vCPU",
                    TIMEOUT_WARNING_INTERVAL_IN_MS * counter
                );
                counter += 1;
            }
        }
        trace!("MicroVM paused");
        // Flush output to linuxd
        self.io_control_tx.send(IoControlResponse::FlushOutput)?;
        // TODO: tell linuxd to stop sending messages (Flushing -> Paused) https://github.com/nanvix/nanvix/issues/945
        // TODO: get a response from linuxd https://github.com/nanvix/nanvix/issues/945
        self.io_control_tx.send(IoControlResponse::FlushInput)?;
        self.receive_linux_daemon_flushed()?;
        self.state = State::Paused;
        trace!("State: Running -> Paused");
        self.io_control_tx.send(IoControlResponse::MicroVmPaused)?;
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a `LinuxDaemonFlushed` message from the control input.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    fn receive_linux_daemon_flushed(&mut self) -> Result<()> {
        // Check how long it takes to receive a response
        let start: Instant = Instant::now();
        let mut counter: usize = 1;
        // Loop until `LinuxDaemonFlushed` arrives.
        // Different kinds of messages can be ignored,
        // as they wouldn't do anything while the VMM is pausing.
        while match self.io_control_rx.try_recv() {
            Ok(command) => command != IoControlCommand::LinuxDaemonFlushed,
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the vmm has disconnected".to_string();
                error!("receive_linux_daemon_flushed(): {reason}");
                anyhow::bail!(reason)
            },
        } {
            // Log a warning and increment the counter every TIMEOUT_WARNING_INTERVAL_IN_MS ms.
            let elapsed_time: usize = start.elapsed().as_millis() as usize;
            if elapsed_time > TIMEOUT_WARNING_INTERVAL_IN_MS * counter {
                warn!(
                    "{}ms have passed waiting for `LinuxDaemonFlushed`",
                    TIMEOUT_WARNING_INTERVAL_IN_MS * counter
                );
                counter += 1;
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
    fn resume_protocol(&mut self) -> Result<()> {
        // Write to the kernel a pause is no longer requested.
        (self.resume_microvm)()?;
        // Tell microvm to resume
        self.vcpu_control_tx.send(VcpuControlCommand::Resume)?;
        self.state = State::Running;
        trace!("State: Paused -> Running");
        Ok(())
    }
}
