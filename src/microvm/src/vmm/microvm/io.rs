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
    _state: OrchestratorState,
    /// Command sender to the VMM.
    _control_input_tx: Sender<ControlCommand>,
    /// Response receiver from the VMM.
    _control_output_rx: Receiver<ControlCommandResponse>,
    // TODO: channels to an outside issuer of snapshot commands and to linuxd.
}

//==================================================================================================
// Enums
//==================================================================================================

///
/// # Description
///
/// States relating to snapshots functionality. Snapshots may be loaded at PreBoot, and created at Paused.
/// TODO: add `Running`, `Pausing`, `PausingAndOutputFlushed`, and `Paused` states.
///
enum OrchestratorState {
    PreBoot,
}

///
/// # Description
///
/// Control plane commands.
/// TODO:
/// Add commands relating to snapshots: `StartMicroVM`, `LoadAndRun`, `ResumeMicroVM`, `PauseMicroVM`, `PauseAndCreateSnapshot`, `LinuxDaemonFlushed`, `CreateSnapshot`, `LoadSnapshot`.
///
pub enum ControlCommand {}

///
/// # Description
///
/// Control plane command responses.
/// TODO:
/// Add `MicroVmPaused` response.
///
pub enum ControlCommandResponse {}

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
            _state: OrchestratorState::PreBoot,
            _control_input_tx: control_input_tx,
            _control_output_rx: control_output_rx,
        })
    }

    ///
    /// # Description
    ///
    /// Runs the I/O thread.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    fn run(&mut self) -> Result<()> {
        let mut round: usize = 0;

        // Cycle through actions to avoid starvation.
        loop {
            if round % 4 == 0 {
                self.try_receive_from_microvm()?;
            } else if round % 4 == 1 {
                self.try_send_to_gateway()?;
            } else if round % 4 == 2 {
                self.try_receive_from_gateway()?;
            } else {
                self.try_send_to_microvm()?;
            }
            round += 1;
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
                // Label: microvm::io::try_recv_from_microvm()
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

                // Label: microvm::io::try_send_to_gateway()
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
                // Label: microvm::io::try_send_to_microvm()
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
}
