// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    Gateway,
    vmm::{
        MicroVm,
        VirtualProcessorHandle,
    },
};
use ::anyhow::Result;
use ::std::{
    collections::VecDeque,
    io::ErrorKind,
    sync::{
        Arc,
        mpsc::{
            Receiver,
            Sender,
            TryRecvError,
        },
        Mutex
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
    /// Connection to the snapshot interface.
    //_snapshot_gateway: Gateway, // where snapshot commands come from
    /// MicroVM handle to issue snapshots.
    _microvm: Arc<Mutex<MicroVm>>,
    /// State in the snapshotting protocol.
    _state: OrchestratorState,
    /// Handles to issue pause / resume commands.
    vcpu_handle: VirtualProcessorHandle,
    /// Channel through which the MicroVM informs it has paused.
    _paused_rx: Receiver<Message>,
}
//==================================================================================================
// Enums
//==================================================================================================

enum OrchestratorState {
    PreBoot,
    Running,
    Pausing,
    PausingAndOutputFlushed,
    Paused,
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
    ///
    /// # Returns
    ///
    /// A handle to the I/O thread.
    ///
    pub fn spawn(
        gateway: Gateway,
        microvm_rx: Receiver<Message>,
        microvm_tx: Sender<Message>,
        microvm: Arc<Mutex<MicroVm>>,
        vcpu_handle: VirtualProcessorHandle,
        paused_rx: Receiver<Message>,
    ) -> JoinHandle<Result<()>> {
        thread::spawn(move || {
            let mut io_thread: IoThread = IoThread::new(
                gateway, microvm_rx, microvm_tx, microvm, vcpu_handle, paused_rx)?;
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
    ///
    /// # Returns
    ///
    /// Upon success, a new I/O thread is returned. Otherwise, an error is returned.
    ///
    fn new(
        gateway: Gateway,
        microvm_rx: Receiver<Message>,
        microvm_tx: Sender<Message>,
        microvm: Arc<Mutex<MicroVm>>,
        vcpu_handle: VirtualProcessorHandle,
        paused_rx: Receiver<Message>,
    ) -> Result<Self> {
        Ok(Self {
            gateway,
            microvm_rx,
            microvm_tx,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            _microvm: microvm,
            _state: OrchestratorState::PreBoot,
            vcpu_handle,
            _paused_rx: paused_rx,
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
        if let Some(thread_handle) = self.vcpu_handle.vcpu_thread.take() {
            while !thread_handle.is_finished() {
                self.try_receive_from_microvm()?;
                self.try_send_to_gateway()?;
                self.try_receive_from_gateway()?;
                self.try_send_to_microvm()?;
            }
        }
        Ok(())
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
                profiler::timestamp_message!(&mut message.payload, std::mem::offset_of!(syscall::LinuxDaemonMessage, payload) + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer));
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
                profiler::timestamp_message!(&mut message_clone.payload, std::mem::offset_of!(syscall::LinuxDaemonMessage, payload) + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer));
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
                profiler::timestamp_message!(&mut message.payload, std::mem::offset_of!(syscall::LinuxDaemonMessage, payload) + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer));
                // NOTE: calling `send()` on a channel does not block.
                self.microvm_tx.send(message)?;
                Ok(())
            },
            None => Ok(()),
        }
    }
}
