// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::io::{
    ControlCommand,
    ControlCommandResponse,
};
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
    io_control_rx: Receiver<ControlCommand>,
    io_control_tx: Sender<ControlCommandResponse>,
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

//==================================================================================================
// Implementations
//==================================================================================================

impl Orchestrator {
    pub fn new(
        io_control_rx: Receiver<ControlCommand>,
        io_control_tx: Sender<ControlCommandResponse>,
        create_snapshot: fn() -> Result<()>,
    ) -> Self {
        Self {
            state: State::PreBoot,
            io_control_rx,
            io_control_tx,
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
                ControlCommand::_StartMicroVm => {
                    if self.state == State::PreBoot {
                        // TODO: separate starting logic from `spawn()` and put it here
                        // This TODO could be done right now, but it's a major refactor.
                        self.state = State::Running;
                        trace!("State: PreBoot -> Running");
                    }
                    Ok(())
                },
                ControlCommand::_LoadSnapshotAndRun => {
                    if self.state == State::PreBoot {
                        // TODO: load snapshot
                        // This TODO requires being able to create snapshots.

                        // The Linux daemon should send messages to PreBoot VMMs by default,
                        // so there's no need to tell it to resume sending messages.

                        if let Err(e) = self.resume_microvm() {
                            let reason: String =
                                format!("LoadSnapshotAndRun: failed to resume microvm: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                        trace!("State: PreBoot -> Running");
                    }
                    Ok(())
                },
                ControlCommand::_PauseMicroVm => {
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
                ControlCommand::_PauseAndCreateSnapshot => {
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
                            .send(ControlCommandResponse::SnapshotCreated)?;
                    }
                    Ok(())
                },
                ControlCommand::_CreateSnapshot => {
                    if self.state == State::Paused {
                        if let Err(e) = (self.create_snapshot)() {
                            let reason: String =
                                format!("CreateSnapshot: failed to create snapshot: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                        trace!("Snapshot created");
                        self.io_control_tx
                            .send(ControlCommandResponse::SnapshotCreated)?;
                    }
                    Ok(())
                },
                ControlCommand::_ResumeMicroVm => {
                    if self.state == State::Paused {
                        // TODO: tell linuxd it's fine to send more messages
                        // This TODO requires having a control plane connection with linuxd
                        if let Err(e) = self.resume_microvm() {
                            let reason: String =
                                format!("ResumeMicroVm: failed to resume microvm: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                        trace!("State: Paused -> Running");
                    }
                    Ok(())
                },
                ControlCommand::LinuxDaemonFlushed => {
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
        // TODO: pause MicroVM (Running -> Paused)
        // and tell linuxd to flush (Running -> Flushing)
        // This TODO requires pausing the vCPU and a control plane communication with linuxd
        trace!("MicroVM paused");
        // Flush output to linuxd
        self.io_control_tx
            .send(ControlCommandResponse::FlushOutput)?;
        // TODO: tell linuxd to stop sending messages (Flushing -> Paused)
        // TODO: get a response from linuxd
        // These TODOs require a control plane communication with linuxd
        self.io_control_tx
            .send(ControlCommandResponse::FlushInput)?;
        self.receive_linux_daemon_flushed()?;
        self.state = State::Paused;
        self.io_control_tx
            .send(ControlCommandResponse::MicroVmPaused)?;
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
            Ok(command) => command != ControlCommand::LinuxDaemonFlushed,
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
    fn resume_microvm(&mut self) -> Result<()> {
        self.state = State::Running;
        trace!("State: Paused -> Running");
        Ok(())
    }
}
