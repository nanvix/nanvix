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
use ::mio::{
    Events,
    Interest,
    Poll,
    Token,
    Waker,
};
use ::std::{
    collections::VecDeque,
    io::ErrorKind,
    mem,
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
// Constants
//==================================================================================================

/// Token represnting an event notification from the control-plane socket.
const CONTROL_PLANE_TOKEN: Token = Token(0);
/// Token represnting an event notification from inbound queues from the VM.
const WAKER_TOKEN: Token = Token(1);
/// Token represnting an event notification from the system VM socket.
const SYSTEM_VM_TOKEN: Token = Token(2);

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// Private data of the I/O thread.
///
pub struct IoThread {
    /// Poll structure to monitor connections and queues.
    poll: Poll,
    /// Waker to notify the I/O thread that it has messages to read from its connected queues.
    waker: Arc<Waker>,
    /// Optional connection to the system VM.
    system_vm_stream: Option<SocketStream>,
    /// Buffer to handle partial reads from the system VM.
    system_vm_partial_read_buffer: VecDeque<u8>,
    /// Optional connection to the external control-plane, nanvixd.
    control_plane_stream: Option<SocketStream>,
    /// Waker token for the memory thread.
    memory_thread_waker: Arc<Waker>,
    /// Gateway receiver.
    data_rx: Receiver<Message>,
    /// Gateway sender.
    data_tx: Sender<Message>,
    /// Queue of incoming messages.
    incoming: VecDeque<Message>,
    /// Queue of outgoing messages.
    outgoing: VecDeque<Message>,
    /// Waker token for the VMM (orchestrator).
    orchestrator_waker: Arc<Waker>,
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
    /// - `system_vm_stream`: Optional connection to the system VM.
    /// - `memory_thread_waker`: Waker token to notify the memory thread when we send in data_tx.
    /// - `data_rx`: MicroVM receiver.
    /// - `data_tx`: MicroVM sender.
    /// - `orchestrator`: Waker token to notify the orchestrator when we send in control_tx.
    /// - `control_tx`: Command sender.
    /// - `control_rx`: Response receiver.
    /// - `control_plane_stream`: Optional connection to the control-plane.
    ///
    /// # Returns
    ///
    /// A handle to the I/O thread.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        system_vm_stream: Option<SocketStream>,
        memory_thread_waker: Arc<Waker>,
        data_rx: Receiver<Message>,
        data_tx: Sender<Message>,
        orchestrator_waker: Arc<Waker>,
        control_tx: Sender<IoControlCommand>,
        control_rx: Receiver<IoControlResponse>,
        control_plane_stream: Option<SocketStream>,
    ) -> Result<(JoinHandle<Result<()>>, Arc<Waker>)> {
        let mut io_thread: IoThread = IoThread::new(
            system_vm_stream,
            memory_thread_waker,
            data_rx,
            data_tx,
            orchestrator_waker,
            control_tx,
            control_rx,
            control_plane_stream,
        )?;
        let io_thread_waker: Arc<Waker> = io_thread.waker();

        let io_thread_handle: JoinHandle<Result<()>> = thread::spawn(move || {
            io_thread.run()?;
            Ok(())
        });

        Ok((io_thread_handle, io_thread_waker))
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
    /// - `memory_thread_waker`: Waker to notify the memory thread of pending messages in data_tx.
    /// - `data_rx`: MicroVM receiver.
    /// - `data_tx`: MicroVM sender.
    /// - `memory_thread_waker`: Waker to notify the orchestrator of pending messages in control_tx.
    /// - `control_tx`: Command sender.
    /// - `control_rx`: Response receiver.
    /// - `control_plane_stream`: Optional connection to the control-plane stream.
    ///
    /// # Returns
    ///
    /// Upon success, a new I/O thread is returned. Otherwise, an error is returned.
    ///
    #[allow(clippy::too_many_arguments)]
    fn new(
        mut system_vm_stream: Option<SocketStream>,
        memory_thread_waker: Arc<Waker>,
        data_rx: Receiver<Message>,
        data_tx: Sender<Message>,
        orchestrator_waker: Arc<Waker>,
        control_tx: Sender<IoControlCommand>,
        control_rx: Receiver<IoControlResponse>,
        mut control_plane_stream: Option<SocketStream>,
    ) -> Result<Self> {
        let poll: Poll = Poll::new()?;

        // Register system VM and/or control-plane streams. At least one should be present
        // otherwise we would not have spawned the I/O thread.
        if let Some(system_vm_stream) = system_vm_stream.as_mut() {
            poll.registry()
                .register(system_vm_stream, SYSTEM_VM_TOKEN, Interest::READABLE)?;
        }
        if let Some(control_plane_stream) = control_plane_stream.as_mut() {
            poll.registry().register(
                control_plane_stream,
                CONTROL_PLANE_TOKEN,
                Interest::READABLE,
            )?;
        }

        // Register a waker token such that other components can notify us about pending work.
        let waker: Arc<Waker> = Arc::new(Waker::new(poll.registry(), WAKER_TOKEN)?);

        Ok(Self {
            poll,
            waker,
            system_vm_stream,
            system_vm_partial_read_buffer: VecDeque::new(),
            memory_thread_waker,
            data_rx,
            data_tx,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            orchestrator_waker,
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
        let mut events: Events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);

        loop {
            self.poll.poll(&mut events, None)?;

            // We must drain each socket/queue until they return WouldBlock in order to not miss
            // any messages. We surface a WouldBlock or a queue being empty with a Break().
            for event in events.iter() {
                match event.token() {
                    // Prioritize events from the control-plane.
                    CONTROL_PLANE_TOKEN => {
                        while self.try_send_to_vmm_control()? != Break(()) {}

                        self.orchestrator_waker.wake()?;
                    },

                    // There are internal events we need to react to in any of our incoming queues.
                    WAKER_TOKEN => {
                        // Try to receive from the I/O thread's control-plane first.
                        while self.try_receive_from_vmm_control()? != Break(()) {}

                        // Try to receive from the data-plane.
                        while self.try_receive_from_microvm()? != Break(()) {}

                        // FIXME (#1025): merge try_receive_from_microvm and try_send_to_system_vm
                        while !self.outgoing.is_empty() {
                            self.try_send_to_system_vm()?;
                        }
                    },

                    SYSTEM_VM_TOKEN => {
                        while self.try_receive_from_system_vm()? != Break(()) {}

                        // FIXME (#1025): merge try_receive_from_system_vm and try_send_to_microvm
                        while !self.incoming.is_empty() {
                            self.try_send_to_microvm()?;
                        }

                        // Notify the memory thread that it has work to do.
                        self.memory_thread_waker.wake()?;
                    },

                    token => {
                        // This error should not happen, but is not fatal, so we log it and
                        // continue.
                        error!(
                            "run(): I/O thread received notification from unexpected token \
                             (token={token:?})"
                        );
                    },
                }
            }
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
    /// Upon success, the received message is pushed into the `incoming` queue, and `Continue()` is
    /// returned. Otherwise, if it would block, `Break()` is returned.
    ///
    fn try_receive_from_system_vm(&mut self) -> Result<ControlFlow<()>> {
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

                        // A partial read corresponds to a WouldBlock, so we break.
                        return Ok(Break(()));
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

                    return Ok(Break(()));
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
            Ok(Continue(()))
        } else {
            Ok(Break(()))
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message from the MicroVM.
    ///
    /// # Returns
    ///
    /// Upon success, the received message is pushed into the `outgoing` queue, and `Continue()` is
    /// returned. Otherwise, if the channel is empty, `Break()` is returned.
    ///
    fn try_receive_from_microvm(&mut self) -> Result<ControlFlow<()>> {
        match self.data_rx.try_recv() {
            Ok(mut message) => {
                profiler::timestamp_message!(
                    &mut message.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                );
                self.outgoing.push_back(message);
                Ok(Continue(()))
            },
            Err(TryRecvError::Empty) => Ok(Break(())),
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
    fn try_send_to_vmm_control(&mut self) -> Result<ControlFlow<()>> {
        if let Some(control_plane_stream) = self.control_plane_stream.as_mut() {
            let cmd: control_plane_api::NanvixdCommand =
                match control_plane_api::recv_command(control_plane_stream) {
                    Ok(cmd) => cmd,
                    // If we would block, return and do nothing.
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Ok(Break(())),
                    Err(e) => {
                        let reason: String = format!(
                            "try_send_to_vmm_control(): failed reading command from control-plane \
                             will shutdown (error={e:?})"
                        );
                        error!("{reason}");

                        // If we encounter an error, shutdown the I/O thread.
                        // FIXME (1004): when we support graceful shutdown of the I/O thread, this
                        // should instead be properly propagated as an error.
                        control_plane_api::NanvixdCommand::Shutdown
                    },
                };
            // Translate nanvixd control-plane commands to internal VMM control commands.
            match cmd {
                control_plane_api::NanvixdCommand::Shutdown => {
                    self.control_tx.send(IoControlCommand::Shutdown)?;
                },
            }
        }

        Ok(Continue(()))
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
    fn try_receive_from_vmm_control(&mut self) -> Result<ControlFlow<()>> {
        match self.control_rx.try_recv() {
            Ok(response) => match response {
                IoControlResponse::FlushOutput => {
                    self.flush_microvm_output()?;
                    Ok(Continue(()))
                },
                IoControlResponse::FlushInput => {
                    self.flush_linuxd_input()?;
                    Ok(Continue(()))
                },
                IoControlResponse::Shutdown => {
                    // TODO (#1004): Break() out of the main loop here.
                    let reason: String = "IO thread shutting down".to_string();
                    anyhow::bail!(reason)
                },
                // TODO: forward to linuxd or nanvixd https://github.com/nanvix/nanvix/issues/945
                _ => Ok(Continue(())),
            },
            Err(TryRecvError::Empty) => Ok(Break(())),
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
        while self.try_receive_from_microvm()? != Break(()) {
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
        while self.try_receive_from_system_vm()? != Break(()) {
            // Keep looping until receiving from the system VM would block, which breaks the loop.
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Get the waker token to notify the I/O thread that there is an event that it must react to.
    /// An event normally means that a message has been pushed to one of its monitored queues.
    ///
    /// # Returns
    ///
    /// Return a handle to the waker object.
    ///
    fn waker(&self) -> Arc<Waker> {
        self.waker.clone()
    }
}
