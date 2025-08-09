// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    Gateway,
    vmm::ControlCommand,
};
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
    /// Command sender to the VMM.
    _vmm_control_tx: Sender<ControlCommand>,
    /// Response receiver from the VMM.
    vmm_control_rx: Receiver<ControlCommandResponse>,
    // TODO: channels to an outside issuer of snapshot commands and to linuxd.
}

//==================================================================================================
// Enums
//==================================================================================================

///
/// # Description
///
/// Control plane command responses.
///
#[derive(PartialEq)]
pub enum ControlCommandResponse {
    MicroVmPaused,
    SnapshotCreated,
    FlushOutput,
    FlushInput,
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
    /// - `vmm_control_tx`: Command sender.
    /// - `vmm_control_rx`: Response receiver.
    ///
    /// # Returns
    ///
    /// A handle to the I/O thread.
    ///
    pub fn spawn(
        gateway: Gateway,
        microvm_rx: Receiver<Message>,
        microvm_tx: Sender<Message>,
        vmm_control_tx: Sender<ControlCommand>,
        vmm_control_rx: Receiver<ControlCommandResponse>,
    ) -> JoinHandle<Result<()>> {
        thread::spawn(move || {
            let mut io_thread: IoThread =
                IoThread::new(gateway, microvm_rx, microvm_tx, vmm_control_tx, vmm_control_rx)?;
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
    /// - `vmm_control_tx`: Command sender.
    /// - `vmm_control_rx`: Response receiver.
    ///
    /// # Returns
    ///
    /// Upon success, a new I/O thread is returned. Otherwise, an error is returned.
    ///
    fn new(
        gateway: Gateway,
        microvm_rx: Receiver<Message>,
        microvm_tx: Sender<Message>,
        vmm_control_tx: Sender<ControlCommand>,
        vmm_control_rx: Receiver<ControlCommandResponse>,
    ) -> Result<Self> {
        Ok(Self {
            gateway,
            microvm_rx,
            microvm_tx,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            _vmm_control_tx: vmm_control_tx,
            vmm_control_rx,
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
            self.try_receive_from_microvm()?;
            self.try_send_to_gateway()?;
            self.try_receive_from_gateway()?;
            self.try_send_to_microvm()?;
            self.try_receive_from_vmm_control()?;
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message from the gateway.
    ///
    /// # Returns
    ///
    /// Upon success, the received message is pushed into the `incoming` queue, and `true` is returned.
    /// Otherwise, if it would block, `false` is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_gateway(&mut self) -> Result<bool> {
        match self.gateway.try_receive() {
            Ok(message) => {
                self.incoming.push_back(message);
                Ok(true)
            },
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    Ok(false)
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
    /// Upon success, the received message is pushed into the `outgoing` queue, and `true`is returned.
    /// Otherwise, if the channel is empty, `false` is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_microvm(&mut self) -> Result<bool> {
        match self.microvm_rx.try_recv() {
            Ok(mut message) => {
                profiler::timestamp_message!(
                    &mut message.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                );
                self.outgoing.push_back(message);
                Ok(true)
            },
            Err(TryRecvError::Empty) => Ok(false),
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

    ///
    /// # Description
    ///
    /// Attempts to receive a response from the VMM.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_vmm_control(&mut self) -> Result<()> {
        match self.vmm_control_rx.try_recv() {
            Ok(response) => match response {
                ControlCommandResponse::FlushOutput => self.flush_microvm_output(),
                ControlCommandResponse::FlushInput => self.flush_linuxd_input(),
                _ => Ok(()), // TODO: forward to whoever is interested. This requires having control channels.
            },
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the vmm has disconnected".to_string();
                // When the guest finishes , the vCPU thread will disconnect from this thread. This
                // situation is normal and should not create an error log.
                anyhow::bail!(reason)
            },
        }
    }

    /// # Description
    ///
    /// Attempts to flush all outstanding output from the MicroVM to the Linux daemon.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn flush_microvm_output(&mut self) -> Result<()> {
        while self.try_receive_from_microvm()? {
            // Keep looping until `microvm_rx` is empty, which breaks the loop.
        }
        while !self.outgoing.is_empty() {
            self.try_send_to_gateway()?;
        }
        Ok(())
    }

    /// # Description
    ///
    /// Attempts to flush all outstanding input from the the Linux daemon to the MicroVM.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn flush_linuxd_input(&mut self) -> Result<()> {
        while self.try_receive_from_gateway()? {
            // Keep looping until receiving from the gateway would block, which breaks the loop.
        }
        Ok(())
    }
}
