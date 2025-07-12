// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    Gateway,
    vmm::microvm::{
        kvm::vcpu::VirtualProcessorHandle,
        microvm::MicroVm
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
    //_snapshot_gateway: Gateway, // where snapshot commands will come from
    /// MicroVM handle to issue snapshots.
    _microvm: Arc<Mutex<MicroVm>>,
    /// State in the snapshotting protocol.
    _state: OrchestratorState,
    /// Handles to issue pause / resume commands.
    pub vcpu_handle: Option<VirtualProcessorHandle>,
    /// Channel through which the MicroVM informs it has paused.
    _paused_rx: Receiver<Message>,
}
//==================================================================================================
// Enums
//==================================================================================================

///
/// # Description
/// 
/// States relating to snapshots functionality.
/// Snapshots may be loaded at PreBoot, and created at Paused.
/// 
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
    /// Creates a new I/O thread.
    ///
    /// # Parameters
    ///
    /// - `gateway`: Connection to gateway.
    /// - `microvm_rx`: MicroVM data receiver.
    /// - `microvm_tx`: MicroVM data sender.
    /// - `microvm`: MicroVM handle to issue snapshots.
    /// - `paused_rx`: MicroVM control channel. Tells the IoThread all vPCUs have paused.
    ///
    /// # Returns
    ///
    /// Upon success, a new I/O thread is returned. Otherwise, an error is returned.
    ///
    pub fn new(
        gateway: Gateway,
        microvm_rx: Receiver<Message>,
        microvm_tx: Sender<Message>,
        microvm: Arc<Mutex<MicroVm>>,
        paused_rx: Receiver<Message>,
    ) -> Self {
        Self {
            gateway,
            microvm_rx,
            microvm_tx,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            _microvm: microvm,
            _state: OrchestratorState::PreBoot,
            vcpu_handle: None,
            _paused_rx: paused_rx,
        }
    }

    ///
    /// # Description
    ///
    /// Runs the I/O thread.
    ///
    /// # Returns
    ///
    /// Upon success, the vCPU exit code is returned. Otherwise, an error is returned instead.
    ///
    pub fn run(&mut self) -> Result<u16> {
        // This function only gets called when both Options are Some.
        if let Some(mut vcpu_handle) = self.vcpu_handle.take() {
            if let Some(thread_handle) = vcpu_handle.vcpu_thread.take() {
                while !thread_handle.is_finished() {
                    self.try_receive_from_microvm()?;
                    self.try_send_to_gateway()?;
                    self.try_receive_from_gateway()?;
                    self.try_send_to_microvm()?;
                }
                match thread_handle.join() {
                    Ok(result) => result,
                    Err(e) => {
                        let reason: String =
                            format!("vcpu_thread error (error={e:?})");
                        error!("run(): {reason}");
                        anyhow::bail!(reason)
                    }
                }
            } else {
                let reason  = "run(): vcpu_handle must have a vcpu_thread handle when running";
                error!("{reason}");
                anyhow::bail!(reason)
            }
        } else {
            let reason  = "run(): IoThread shouldn't run without a vcpu_handle";
            error!("{reason}");
            anyhow::bail!(reason)
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
            Ok(message) => {
                self.outgoing.push_back(message);
                Ok(())
            },
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the microvm has disconnected".to_string();
                error!("try_receive_from_microvm(): {reason}");
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
                let message_clone: Message = message.clone();
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
            Some(message) => {
                // NOTE: calling `send()` on a channel does not block.
                self.microvm_tx.send(message)?;
                Ok(())
            },
            None => Ok(()),
        }
    }
}
