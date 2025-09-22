// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::orchestrator::{
    IoControlCommand,
    IoControlResponse,
};
use ::anyhow::Result;
use ::control_plane_api;
use ::std::{
    collections::VecDeque,
    io::ErrorKind,
    mem,
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
use ::syscomm::SocketStream;
use ::syslog::{
    debug,
    error,
};

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// Private data of the I/O thread.
///
pub struct IoThread {
    /// Optional connection to the system VM.
    system_vm_stream: Option<SocketStream>,
    /// Buffer to handle partial reads from the system VM.
    system_vm_partial_read_buffer: VecDeque<u8>,
    /// Optional connection to the external control-plane, nanvixd.
    control_plane_stream: Option<SocketStream>,
    /// Gateway receiver.
    data_rx: Receiver<Message>,
    /// Gateway sender.
    data_tx: Sender<Message>,
    /// Queue of incoming messages.
    incoming: VecDeque<Message>,
    /// Queue of outgoing messages.
    outgoing: VecDeque<Message>,
    /// Command sender to the VMM.
    control_tx: Sender<IoControlCommand>,
    /// Response receiver from the VMM.
    control_rx: Receiver<IoControlResponse>,
    // TODO: channels to linuxd and nanvixd https://github.com/nanvix/nanvix/issues/945
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
    /// - `system_vm_stream`: Connection to the system VM.
    /// - `data_rx`: MicroVM receiver.
    /// - `data_tx`: MicroVM sender.
    /// - `control_tx`: Command sender.
    /// - `control_rx`: Response receiver.
    ///
    /// # Returns
    ///
    /// A handle to the I/O thread.
    ///
    pub fn spawn(
        system_vm_stream: Option<SocketStream>,
        data_rx: Receiver<Message>,
        data_tx: Sender<Message>,
        control_tx: Sender<IoControlCommand>,
        control_rx: Receiver<IoControlResponse>,
        control_plane_stream: Option<SocketStream>,
    ) -> JoinHandle<Result<()>> {
        thread::spawn(move || {
            let mut io_thread: IoThread = IoThread::new(
                system_vm_stream,
                data_rx,
                data_tx,
                control_tx,
                control_rx,
                control_plane_stream,
            )?;
            io_thread.run()?;
            Ok(())
        })
    }

    ///
    /// # Description
    ///
    /// Creates a new I/O thread. We start an I/O thread if we have either a connection to the
    /// gateway, one to the control plane, or both.
    ///
    /// # Parameters
    ///
    /// - `system_vm_stream`: Optional connection to the system VM.
    /// - `data_rx`: MicroVM receiver.
    /// - `data_tx`: MicroVM sender.
    /// - `control_tx`: Command sender.
    /// - `control_rx`: Response receiver.
    /// - `control_plane_stream`: Optional connection to the control-plane stream.
    ///
    /// # Returns
    ///
    /// Upon success, a new I/O thread is returned. Otherwise, an error is returned.
    ///
    fn new(
        system_vm_stream: Option<SocketStream>,
        data_rx: Receiver<Message>,
        data_tx: Sender<Message>,
        control_tx: Sender<IoControlCommand>,
        control_rx: Receiver<IoControlResponse>,
        control_plane_stream: Option<SocketStream>,
    ) -> Result<Self> {
        Ok(Self {
            system_vm_stream,
            system_vm_partial_read_buffer: VecDeque::new(),
            data_rx,
            data_tx,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            control_tx,
            control_rx,
            control_plane_stream,
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
            self.try_send_to_vmm_control()?;
            self.try_receive_from_microvm()?;
            self.try_send_to_system_vm()?;
            self.try_receive_from_system_vm()?;
            self.try_send_to_microvm()?;
            self.try_receive_from_vmm_control()?;
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message from the system VM. We need to be careful to handle partial
    /// reads properly.
    ///
    /// # Returns
    ///
    /// Upon success, the received message is pushed into the `incoming` queue, and `true` is returned.
    /// Otherwise, if it would block, `false` is returned. Otherwise, an error is returned.
    ///
    fn try_receive_from_system_vm(&mut self) -> Result<bool> {
        if let Some(system_vm_stream) = self.system_vm_stream.as_mut() {
            let mut buf: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];

            let mut num_filled: usize = 0;
            if !self.system_vm_partial_read_buffer.is_empty() {
                // Prepare data in buffer for partial read.
                self.system_vm_partial_read_buffer.make_contiguous();
                let partial_bytes: &[u8] = self.system_vm_partial_read_buffer.as_slices().0;

                // We take the minimum at the end just in case, but the partial read should always be
                // strictly smaller than the message size.
                let num_partial_read: usize = partial_bytes.len().min(buf.len());

                buf[..num_partial_read].copy_from_slice(&partial_bytes[..num_partial_read]);

                // Clear partial read buffer.
                self.system_vm_partial_read_buffer.clear();
                num_filled += num_partial_read;
            }
            // Post-condition: partial_read_buffer is empty.

            match system_vm_stream.try_read_exact(&mut buf[num_filled..]) {
                Ok(n) => {
                    // Handle partial reads by copying all we have read to the partial read buffer and
                    // returning a WouldBlock indicating that we need more data.
                    if n + num_filled < buf.len() {
                        self.system_vm_partial_read_buffer
                            .extend(&buf[..(n + num_filled)]);

                        // A partial read corresponds to a WouldBlock, so we return false.
                        return Ok(false);
                    }
                },
                // If WouldBlock, return false.
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // Handle the situation where we may have filled a partial read from a previous
                    // attempt, but then error-ed out with WouldBlock.
                    if num_filled > 0 {
                        self.system_vm_partial_read_buffer
                            .extend(&buf[..num_filled]);
                    }

                    return Ok(false);
                },
                Err(e) => {
                    let reason: String =
                        format!("failed to receive message from the system VM (error={e:?})");
                    error!("try_receive_from_system_vm(): {reason}");

                    return Err(anyhow::anyhow!(reason));
                },
            };

            let mut message: Message = match Message::try_from_bytes(buf) {
                Ok(message) => message,
                Err(e) => {
                    let reason: String =
                        format!("failed to parse message from system VM (error={e:?})");
                    error!("try_receive_from_system_vm(): {reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            };
            profiler::timestamp_message!(
                &mut message.payload,
                mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                    + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
            );

            self.incoming.push_back(message);
            Ok(true)
        } else {
            Ok(false)
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
        match self.data_rx.try_recv() {
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
    /// Attempts to send a message to the system VM.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn try_send_to_system_vm(&mut self) -> Result<()> {
        if let Some(system_vm_stream) = self.system_vm_stream.as_mut() {
            match self.outgoing.pop_front() {
                Some(message) => {
                    let mut message_clone: Message = message.clone();
                    profiler::timestamp_message!(
                        &mut message_clone.payload,
                        std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                            + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                    );
                    match system_vm_stream.write_all(&message_clone.to_bytes()) {
                        Ok(()) => Ok(()),
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            self.outgoing.push_front(message);
                            Ok(())
                        },
                        Err(e) => {
                            let reason: String =
                                format!("failed to send message to the system VM (error={e:?})");
                            error!("try_send_to_system_vm(): {reason}");
                            Err(anyhow::anyhow!(reason))
                        },
                    }
                },
                None => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to read a message from the external control-plane (i.e. nanvixd) and forwards it
    /// to the VMM control-plane.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn try_send_to_vmm_control(&mut self) -> Result<()> {
        if let Some(control_plane_stream) = self.control_plane_stream.as_mut() {
            let cmd: control_plane_api::Command =
                match control_plane_api::try_read_command(control_plane_stream) {
                    Ok(cmd) => cmd,
                    // If we would block, return and do nothing.
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
                    Err(e) => {
                        let reason: String = format!(
                            "try_send_to_vmm_control(): failed reading command from control-plane \
                             (error={e:?})"
                        );
                        error!("{reason}");
                        return Err(anyhow::anyhow!(reason));
                    },
                };
            // Translate nanvixd control-plane commands to internal VMM control commands.
            match cmd {
                control_plane_api::Command::Shutdown => {
                    self.control_tx.send(IoControlCommand::Shutdown)?;
                },
            }
        }

        Ok(())
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
                self.data_tx.send(message)?;
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
        match self.control_rx.try_recv() {
            Ok(response) => match response {
                IoControlResponse::FlushOutput => self.flush_microvm_output(),
                IoControlResponse::FlushInput => self.flush_linuxd_input(),
                IoControlResponse::Shutdown => {
                    // TODO (#1004): Break() out of the main loop here.
                    let reason: String = "IO thread shutting down".to_string();
                    anyhow::bail!(reason)
                },
                _ => Ok(()), // TODO: forward to linuxd or nanvixd https://github.com/nanvix/nanvix/issues/945
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
            // Keep looping until `data_rx` is empty, which breaks the loop.
        }
        while !self.outgoing.is_empty() {
            self.try_send_to_system_vm()?;
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
        while self.try_receive_from_system_vm()? {
            // Keep looping until receiving from the system VM would block, which breaks the loop.
        }
        Ok(())
    }
}
