// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::Gateway;
use ::anyhow::Result;
use ::std::{
    collections::VecDeque,
    io::ErrorKind,
    sync::mpsc::{
        Receiver,
        Sender,
        TryRecvError,
    },
    thread::{
        self,
        JoinHandle,
    },
};
use ::sys::ipc::Message;

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// Private data of the I/O thread.
///
pub struct IoThread {
    /// Connection to the gateway.
    gateway: Gateway,
    /// Gateway receiver.
    microvm_rx: Receiver<Message>,
    /// Gateway sender.
    microvm_tx: Sender<Message>,
    /// Queue of incoming messages.
    incoming: VecDeque<Message>,
    /// Queue of outgoing messages.
    outgoing: VecDeque<Message>,
    /// State in the snapshotting protocol.
    state: OrchestratorState,
    /// Command sender to the VMM.
    control_input_tx: Sender<ControlCommand>,
    /// Response receiver from the VMM.
    control_output_rx: Receiver<ControlCommandResponse>,
    // TODO: channels to an outside issuer of snapshot commands and to linuxd.
}

//==================================================================================================
// Enums
//==================================================================================================

///
/// # Description
///
/// States relating to snapshots functionality. Snapshots may be loaded at PreBoot, and created at Paused.
///
#[derive(PartialEq)]
enum OrchestratorState {
    PreBoot,
    Running,
    Pausing,
    PausingAndOutputFlushed,
    Paused,
}

///
/// # Description
///
/// Control plane commands.
/// TODO:
/// Add commands relating to snapshots: `StartMicroVM`, `LoadAndRun`, `PauseAndCreateSnapshot`, `LinuxDaemonFlushed`, `LoadSnapshot`.
///
#[derive(PartialEq)]
pub enum ControlCommand {
    PauseMicroVm,
    CreateSnapshot,
    ResumeMicroVm,
}

///
/// # Description
///
/// Control plane command responses.
///
#[derive(PartialEq)]
pub enum ControlCommandResponse {
    Empty, // NOTE: `Empty` is probably a bad implementation. There must be a better way.
    MicroVmPaused,
    SnapshotCreated,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl IoThread {
    ///
    /// # Description
    ///
    /// Spawns a new I/O thread.
    ///
    /// # Parameters
    ///
    /// - `gateway`: Connection to gateway.
    /// - `microvm_rx`: MicroVM receiver.
    /// - `microvm_tx`: MicroVM sender.
    /// - `control_input_tx`: Command sender.
    /// - `control_output_rx`: Response receiver.
    ///
    /// # Returns
    ///
    /// A handle to the I/O thread.
    ///
    pub fn spawn(
        gateway: Gateway,
        microvm_rx: Receiver<Message>,
        microvm_tx: Sender<Message>,
        control_input_tx: Sender<ControlCommand>,
        control_output_rx: Receiver<ControlCommandResponse>,
    ) -> JoinHandle<Result<()>> {
        thread::spawn(move || {
            let mut io_thread: IoThread = IoThread::new(
                gateway,
                microvm_rx,
                microvm_tx,
                control_input_tx,
                control_output_rx,
            )?;
            io_thread.run()?;
            Ok(())
        })
    }

    ///
    /// # Description
    ///
    /// Creates a new I/O thread.
    ///
    /// # Parameters
    ///
    /// - `gateway`: Connection to gateway.
    /// - `microvm_rx`: MicroVM receiver.
    /// - `microvm_tx`: MicroVM sender.
    /// - `control_input_tx`: Command sender.
    /// - `control_output_rx`: Response receiver.
    ///
    /// # Returns
    ///
    /// Upon success, a new I/O thread is returned. Otherwise, an error is returned.
    ///
    fn new(
        gateway: Gateway,
        microvm_rx: Receiver<Message>,
        microvm_tx: Sender<Message>,
        control_input_tx: Sender<ControlCommand>,
        control_output_rx: Receiver<ControlCommandResponse>,
    ) -> Result<Self> {
        Ok(Self {
            gateway,
            microvm_rx,
            microvm_tx,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            state: OrchestratorState::PreBoot,
            control_input_tx,
            control_output_rx,
        })
    }

    ///
    /// # Description
    ///
    /// Runs the I/O thread according to the state in the snapshotting protocol state machine.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    fn run(&mut self) -> Result<()> {
        loop {
            match self.state {
                // In `PreBoot` state, figure out whether to load a snapshot or run a fresh run.
                OrchestratorState::PreBoot => {
                    self.try_receive_from_snapshot_client()?;
                },
                // In `Running` state, execute the application and check if it should pause.
                OrchestratorState::Running => {
                    self.try_receive_from_microvm()?;
                    self.try_send_to_gateway()?;
                    self.try_receive_from_gateway()?;
                    self.try_send_to_microvm()?;
                    self.try_receive_from_snapshot_client()?;
                },
                // In `Pausing` state, pause the MicroVM and flush all outstanding output.
                // Flushing the output means it doesn't need to be saved in snapshots.
                OrchestratorState::Pausing => {
                    self.control_input_tx.send(ControlCommand::PauseMicroVm)?;
                    while self.try_receive_from_control_output()?
                        != ControlCommandResponse::MicroVmPaused
                    {
                        self.flush_microvm_output()?;
                    }
                    self.flush_microvm_output()?;
                    // TODO:
                    // `send` to linuxd `output flushed` so it can advance the snapshotting protocol.
                    self.state = OrchestratorState::PausingAndOutputFlushed;
                },
                // In the `PausingAndOutputFlushed` state, wait for the response from the Linux Daemon,
                // ensuring any outstanding messages are either buffered in `incoming`, or are in `linuxd`.
                OrchestratorState::PausingAndOutputFlushed => {
                    // TODO:
                    // Try to receive `LinuxDaemonFlushed` from linuxd to advance the snapshotting protocol.
                    // Also try to receive data from linuxd while the `LinuxDaemonFlushed` response doesn't arrive.
                    // Should be something like the while-loop from the `Pausing` state.

                    // Flush the incoming messages into the buffer.
                    // NOTE: this match-statement is nearly identical to `try_receive_from_gateway()`.
                    // It could be substituted if that method returned a Result<bool> instead.
                    loop {
                        match self.gateway.try_receive() {
                            Ok(message) => self.incoming.push_back(message),
                            Err(e) => {
                                if e.kind() == ErrorKind::WouldBlock {
                                    break;
                                } else {
                                    let reason: String = format!(
                                        "failed to receive message from the gateway (error={e:?})"
                                    );
                                    error!("try_receive_from_gateway(): {reason}");
                                    anyhow::bail!(reason)
                                }
                            },
                        }
                    }
                    self.state = OrchestratorState::Paused;
                },
                // In the `Paused` state, wait for commands to create a snapshot, resume execution, or something else (migration, kill VM).
                OrchestratorState::Paused => {
                    // NOTE: make this a blocking call instead (another method without `try` in the name).
                    self.try_receive_from_snapshot_client()?;
                },
            }
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message from the gateway.
    ///
    /// # Returns
    ///
    /// Upon success, the received message is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_gateway(&mut self) -> Result<()> {
        match self.gateway.try_receive() {
            Ok(message) => {
                self.incoming.push_back(message);
                Ok(())
            },
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    Ok(())
                } else {
                    let reason: String =
                        format!("failed to receive message from the gateway (error={e:?})");
                    error!("try_receive_from_gateway(): {reason}");
                    anyhow::bail!(reason)
                }
            },
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message from the MicroVM.
    ///
    /// # Returns
    ///
    /// Upon success, the received message is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_microvm(&mut self) -> Result<()> {
        match self.microvm_rx.try_recv() {
            Ok(mut message) => {
                profiler::timestamp_message!(
                    &mut message.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                );
                self.outgoing.push_back(message);
                Ok(())
            },
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the microvm has disconnected".to_string();
                // When the guest finishes , the vCPU thread will disconnect from this thread. This
                // situation is normal and should not create an error log.
                debug!("try_receive_from_microvm(): {reason}");
                anyhow::bail!(reason)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to send a message to the gateway.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn try_send_to_gateway(&mut self) -> Result<()> {
        match self.outgoing.pop_front() {
            Some(message) => {
                let mut message_clone: Message = message.clone();
                profiler::timestamp_message!(
                    &mut message_clone.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                );
                match self.gateway.try_send(message_clone) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        if e.kind() == ErrorKind::WouldBlock {
                            self.outgoing.push_front(message);
                            Ok(())
                        } else {
                            let reason: String =
                                format!("failed to send message to the gateway (error={e:?})");
                            error!("try_send_to_gateway(): {reason}");
                            anyhow::bail!(reason)
                        }
                    },
                }
            },
            None => Ok(()),
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to send a message to the MicroVM.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn try_send_to_microvm(&mut self) -> Result<()> {
        match self.incoming.pop_front() {
            Some(mut message) => {
                profiler::timestamp_message!(
                    &mut message.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                );
                // NOTE: calling `send()` on a channel does not block.
                self.microvm_tx.send(message)?;
                Ok(())
            },
            None => Ok(()),
        }
    }

    fn try_receive_from_snapshot_client(&mut self) -> Result<()> {
        // Placeholder match-statement to simulate a client
        match self.state {
            // TODO:
            // Receive from a client whether to load a snapshot or run a fresh binary.
            // Loading transitions to `Paused`, running transitions to `Running`.
            // A `load` command with a `run` flag transitions from `Paused` to `Running` automatically.
            OrchestratorState::PreBoot => self.state = OrchestratorState::Running,
            // TODO: transition to `Pausing` if a `Pause` command arrives.
            // If the `Pause` command includes a `create snapshot` flag,
            // then store a boolean to avoid receiving an extra command in the `Paused` state.
            OrchestratorState::Running => self.state = OrchestratorState::Pausing,
            OrchestratorState::Pausing => {
                unreachable!("This method is not called while in `Pausing` state.")
            },
            OrchestratorState::PausingAndOutputFlushed => {
                unreachable!("This method is not called while in `PausingAndOutputFlushed` state.")
            },
            // TODO:
            // Create snapshot OR transition to `Running`. Currently does both to check the codepaths.
            OrchestratorState::Paused => {
                self.control_input_tx.send(ControlCommand::CreateSnapshot)?;
                // NOTE: make this a blocking call with no loop to avoid polling.
                loop {
                    if self.try_receive_from_control_output()?
                        == ControlCommandResponse::SnapshotCreated
                    {
                        break;
                    }
                }
                self.control_input_tx.send(ControlCommand::ResumeMicroVm)?;
                // NOTE: there's no need for a `Resumed` response.
                self.state = OrchestratorState::Running;
            },
        }
        Ok(())
    }

    /// # Description
    ///
    /// Attempts to load a snapshot of the MicroVM.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn flush_microvm_output(&mut self) -> Result<()> {
        loop {
            // NOTE: this match-statement is nearly identical to `try_receive_from_microvm()`.
            // It could be substituted if that method returned a Result<bool> instead.
            match self.microvm_rx.try_recv() {
                Ok(mut message) => {
                    profiler::timestamp_message!(
                        &mut message.payload,
                        std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                            + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                    );
                    self.outgoing.push_back(message)
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    let reason: String = "the microvm has disconnected".to_string();
                    // When the guest finishes , the vCPU thread will disconnect from this thread. This
                    // situation is normal and should not create an error log.
                    debug!("try_receive_from_microvm(): {reason}");
                    anyhow::bail!(reason)
                },
            }
        }
        while !self.outgoing.is_empty() {
            self.try_send_to_gateway()?;
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a response from the VMM.
    ///
    /// # Returns
    ///
    /// Upon success, the received response is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_control_output(&mut self) -> Result<ControlCommandResponse> {
        match self.control_output_rx.try_recv() {
            Ok(response) => Ok(response),
            Err(TryRecvError::Empty) => Ok(ControlCommandResponse::Empty),
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the vmm has disconnected".to_string();
                // When the guest finishes , the vCPU thread will disconnect from this thread. This
                // situation is normal and should not create an error log.
                anyhow::bail!(reason)
            },
        }
    }
}
