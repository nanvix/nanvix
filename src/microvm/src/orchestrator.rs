// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::std::{
    sync::mpsc::{
        Receiver,
        Sender,
        TryRecvError,
    },
    time::Instant,
};

//==================================================================================================
// Constants
//==================================================================================================

// This value was chosen so it catches issues without polluting the logs with too many warnings.
pub const TIMEOUT_WARNING_INTERVAL_IN_MS: usize = 10;

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// Channels used for orchestrating control commands and the state in the snapshotting protocol.
///
pub struct Orchestrator {
    state: State,
    io_control_rx: Receiver<IoControlCommand>,
    io_control_tx: Sender<IoControlResponse>,
    memory_control_rx: Receiver<MemoryControlResponse>,
    memory_control_tx: Sender<MemoryControlCommand>,
    vcpu_control_rx: Receiver<VcpuControlResponse>,
    vcpu_control_tx: Sender<VcpuControlCommand>,
    create_snapshot: fn() -> Result<()>,
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
}

///
/// # Description
///
/// Control plane commands from the VMM to the memory thread.
///
#[derive(PartialEq)]
pub enum MemoryControlCommand {
    Pause,
    Resume,
}

///
/// # Description
///
/// Control plane command responses from the memory thread to the VMM.
///
#[derive(PartialEq)]
pub enum MemoryControlResponse {
    PauseError,
    ResumeError,
    ResumeWritten,
}

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
    Tid(u64),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Orchestrator {
    pub fn new(
        io_control_rx: Receiver<IoControlCommand>,
        io_control_tx: Sender<IoControlResponse>,
        memory_control_rx: Receiver<MemoryControlResponse>,
        memory_control_tx: Sender<MemoryControlCommand>,
        vcpu_control_rx: Receiver<VcpuControlResponse>,
        vcpu_control_tx: Sender<VcpuControlCommand>,
        create_snapshot: fn() -> Result<()>,
    ) -> Self {
        Self {
            state: State::PreBoot,
            io_control_rx,
            io_control_tx,
            memory_control_rx,
            memory_control_tx,
            vcpu_control_rx,
            vcpu_control_tx,
            create_snapshot,
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to handle a command from the control input.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    pub fn handle_command(&mut self) -> Result<()> {
        match self.io_control_rx.try_recv() {
            Ok(command) => match command {
                IoControlCommand::_StartMicroVm => {
                    if self.state == State::PreBoot {
                        // TODO: separate starting logic from `spawn()` and put it here
                        // This only makes sense when snapshots can already be loaded https://github.com/nanvix/nanvix/issues/948
                        self.state = State::Running;
                        trace!("State: PreBoot -> Running");
                    }
                    Ok(())
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
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                    }
                    Ok(())
                },
                IoControlCommand::_PauseMicroVm => {
                    if self.state == State::Running {
                        if let Err(e) = self.pause_protocol() {
                            let reason: String =
                                format!("PauseMicroVm: failed to pause microvm: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                    }
                    Ok(())
                },
                IoControlCommand::_PauseAndCreateSnapshot => {
                    if self.state == State::Running {
                        if let Err(e) = self.pause_protocol() {
                            let reason: String =
                                format!("PauseAndCreateSnapshot: failed to pause microvm: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                        if let Err(e) = (self.create_snapshot)() {
                            let reason: String =
                                format!("PauseAndCreateSnapshot: failed to create snapshot: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                        trace!("Snapshot created");
                        self.io_control_tx
                            .send(IoControlResponse::SnapshotCreated)?;
                    }
                    Ok(())
                },
                IoControlCommand::_CreateSnapshot => {
                    if self.state == State::Paused {
                        if let Err(e) = (self.create_snapshot)() {
                            let reason: String =
                                format!("CreateSnapshot: failed to create snapshot: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                        trace!("Snapshot created");
                        self.io_control_tx
                            .send(IoControlResponse::SnapshotCreated)?;
                    }
                    Ok(())
                },
                IoControlCommand::_ResumeMicroVm => {
                    if self.state == State::Paused {
                        // TODO: tell linuxd it's fine to send more messages https://github.com/nanvix/nanvix/issues/945
                        if let Err(e) = self.resume_protocol() {
                            let reason: String =
                                format!("ResumeMicroVm: failed to resume microvm: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                    }
                    Ok(())
                },
                IoControlCommand::LinuxDaemonFlushed => {
                    // NOTE: this will be unreachable once the communication is fully implemented
                    // `LinuxDaemonFlushed` should only be sent in the middle of `pause_protocol`.
                    // In fact, it should already be unreachable, but it cannot be tested ATM.
                    Ok(())
                },
            },
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => {
                let reason: String =
                    ("disconnected from the input control command channel").to_string();
                error!("handle_command(): {reason}");
                anyhow::bail!(reason);
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
        if let Err(e) = self.memory_control_tx.send(MemoryControlCommand::Pause) {
            let reason: String =
                format!("error sending `Pause` to the memory thread (error={e:?})");
            error!("pause_protocol(): {reason}");
            anyhow::bail!(reason)
        }
        // Wait for the MicroVM to confirm it has paused.
        let start: Instant = Instant::now();
        let mut counter: usize = 1;
        loop {
            match self.vcpu_control_rx.try_recv() {
                Ok(VcpuControlResponse::Paused) => break,
                Ok(VcpuControlResponse::Tid(tid)) => unreachable!(
                    "The tid is only sent when the vCPU thread is spawned. Sending it in the \
                     middle of a pause protocol is against the protocol. tid=({tid})"
                ),
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
                match self.memory_control_rx.try_recv() {
                    Ok(MemoryControlResponse::PauseError) => todo!(), // TODO: graceful shutdown https://github.com/nanvix/nanvix/issues/949
                    Ok(MemoryControlResponse::ResumeError) => unreachable!(
                        "Received `ResumeError` during pause protocol: this indicates a protocol \
                         violation, as `ResumeError` should not be sent while pausing."
                    ),
                    Ok(MemoryControlResponse::ResumeWritten) => unreachable!(
                        "Received `ResumeWritten` during pause protocol: this indicates a \
                         protocol violation, as `ResumeWritten` should not be sent while pausing."
                    ),
                    Err(TryRecvError::Empty) => (),
                    Err(TryRecvError::Disconnected) => {
                        let reason: String = "the vmm has disconnected".to_string();
                        error!("pause_protocol(): {reason}");
                        anyhow::bail!(reason)
                    },
                }
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
        // Tell memory thread to write to the kernel
        self.memory_control_tx.send(MemoryControlCommand::Resume)?;
        // Wait for confirmation
        let start: Instant = Instant::now();
        let mut counter: usize = 1;
        loop {
            match self.memory_control_rx.try_recv() {
                Ok(MemoryControlResponse::ResumeWritten) => break,
                Ok(MemoryControlResponse::ResumeError) => todo!(), // TODO: graceful shutdown https://github.com/nanvix/nanvix/issues/949
                Ok(MemoryControlResponse::PauseError) => {
                    unreachable!(
                        "Received `PauseError` during resume protocol: this indicates a protocol \
                         violation, as `PauseError` should not be sent while resuming."
                    )
                },
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => {
                    let reason: String = "the memory thread has disconnected".to_string();
                    error!("resume_protocol(): {reason}");
                    anyhow::bail!(reason)
                },
            }
            // Log a warning and increment the counter every TIMEOUT_WARNING_INTERVAL_IN_MS ms.
            let elapsed_time: usize = start.elapsed().as_millis() as usize;
            if elapsed_time > TIMEOUT_WARNING_INTERVAL_IN_MS * counter {
                warn!(
                    "{}ms have passed waiting for `ResumeWritten`",
                    TIMEOUT_WARNING_INTERVAL_IN_MS * counter
                );
                counter += 1;
            }
        }
        // Tell microvm to resume
        self.vcpu_control_tx.send(VcpuControlCommand::Resume)?;
        self.state = State::Running;
        trace!("State: Paused -> Running");
        Ok(())
    }
}
