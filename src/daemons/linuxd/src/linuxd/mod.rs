// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod assemble;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::RequestAssembler,
    user_vm_handle::UserVmHandle,
    venv::{
        VenvCommand,
        VirtualEnviromentDirectory,
    },
    worker_thread::WorkerThreadHandle,
};
use ::anyhow::Result;
use ::mio::{
    Events,
    Interest,
    Poll,
    Token,
};
use ::nanvixd::control_plane;
use ::std::{
    collections::VecDeque,
    io::ErrorKind,
    sync::{
        mpsc::{
            Receiver,
            Sender,
        },
        Arc,
        Mutex,
        MutexGuard,
    },
    time::Duration,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::syscall::venv::VirtualEnvironmentIdentifier;
use ::syscomm::{
    SocketListener,
    SocketStream,
};

//==================================================================================================
// Constants
//==================================================================================================

/// We use ID 0 for the control-plane socket in the main poll structure.
const CONTROL_PLANE_CONNECTION_ID: usize = 0;
/// We use ID 1 for the user VM connection in the main poll structure.
const DEFAULT_CONNECTION_ID: usize = 1;
/// We use ID 0 for the gateway listener socket in the gateway poll structure. Given that this
/// socket is monitored in a different poll, we can re-use the connection ID 0.
const GATEWAY_LISTENER_CONNECTION_ID: usize = 0;

//==================================================================================================
// Structures
//==================================================================================================

pub struct LinuxDaemon {
    assembler: Arc<Mutex<RequestAssembler>>,
    control_plane_stream: SocketStream,
    uvm_stream: SocketStream,
    gateway_listener: Option<SocketListener>,
    venv: Arc<Mutex<VirtualEnviromentDirectory>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    pub fn init(
        control_plane_stream: SocketStream,
        uvm_stream: SocketStream,
        gateway_listener: Option<SocketListener>,
    ) -> Result<Self, Error> {
        Ok(Self {
            assembler: Arc::new(Mutex::new(RequestAssembler::default())),
            control_plane_stream,
            uvm_stream,
            gateway_listener,
            venv: Arc::new(Mutex::new(VirtualEnviromentDirectory::new())),
        })
    }

    /// Helper method to close a connection to a user VM identified by the connection id. Closing
    /// the connection also involves stopping all associated worker threads.
    fn close_connection(
        uvm_handle: UserVmHandle,
        poll: &Poll,
        worker_threads: Option<VecDeque<WorkerThreadHandle>>,
    ) {
        // De-register the user VM socket from the poll structure.
        let user_vm_stream: Arc<Mutex<(SocketStream, VecDeque<u8>)>> =
            uvm_handle.get_user_vm_stream();
        match user_vm_stream.lock() {
            Ok(mut guard) => {
                let (locked_uvm_stream, _): &mut (SocketStream, VecDeque<u8>) = &mut guard;
                if let Err(e) = poll.registry().deregister(locked_uvm_stream) {
                    error!("failed to deregister user VM from poll (error={e:?})");
                }
            },
            Err(e) => {
                error!("error acquiring lock on user VM stream (error={e:?})");
            },
        };

        // Send a shutdown message to all worker threads associated
        // with this user VM.
        if let Some(mut worker_threads) = worker_threads {
            for worker_thread in worker_threads.drain(..) {
                trace!("sending interrupt to worker thread (thread_id={:?})", worker_thread.id);

                // Each worker thread may be in one of three states:
                // 1. Running
                // 2. Blocked on a system call
                // 3. Blocked waiting for a new message from the channel
                //
                // To gracefully shutdown the thread, we enqueue a shutdown message to the
                // message channel. In case the thread is blocked on a system call, we also
                // send it an interrupt signal and handle EINTR accordingly. Note that a signal
                // interrupt will not unblock a thread waiting on a queue, so we need both
                // mechanisms.
                //
                // If any of the commands fail, continue trying to drain the remaining
                // threads.
                if let Err(e) = worker_thread.cmd_tx.send(VenvCommand::Shutdown) {
                    error!(
                        "error sending shutdown command to worker thread (thread_id={:?}, \
                         error={e:?})",
                        worker_thread.id
                    );
                }
                if let Err(e) = worker_thread.stop() {
                    error!(
                        "error sending interrupt to worker thread (thread_id={:?}, error={e:?})",
                        worker_thread.id
                    );
                }
                if let Err(e) = worker_thread.handle.join() {
                    error!(
                        "error joining worker thread (thread_id={:?}, error={e:?})",
                        worker_thread.id
                    );
                }
            }
        }
    }

    fn log_and_error(code: ErrorCode, msg: &'static str) -> Error {
        error!("{msg}");
        Error::new(code, msg)
    }

    pub fn run(mut self) -> Result<(), Error> {
        const CONTROL_PLANE_TOKEN: Token = Token(CONTROL_PLANE_CONNECTION_ID);
        const USER_VM_TOKEN: Token = Token(DEFAULT_CONNECTION_ID);
        const GATEWAY_LISTENER_TOKEN: Token = Token(GATEWAY_LISTENER_CONNECTION_ID);

        let mut events: Events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);

        // Poll structure monitoring the user VM and control-plane connections.
        let mut user_vm_poll: Poll = Poll::new()
            .map_err(|_| Self::log_and_error(ErrorCode::IoErr, "failed to create Poll"))?;
        user_vm_poll
            .registry()
            .register(&mut self.control_plane_stream, CONTROL_PLANE_TOKEN, Interest::READABLE)
            .map_err(|_| {
                Self::log_and_error(ErrorCode::IoErr, "failed to register control-plane to poll")
            })?;
        user_vm_poll
            .registry()
            .register(&mut self.uvm_stream, USER_VM_TOKEN, Interest::READABLE)
            .map_err(|_| {
                Self::log_and_error(ErrorCode::IoErr, "failed to register user VM listener to poll")
            })?;

        // Poll structure used to accept connections from the gateway in a blocking fashion. For
        // the gateway we want to give each worker thread the mutex-protected socket stream
        // connected to the gateway.
        let gateway_poll: Option<Poll> =
            if let Some(gateway_listener) = self.gateway_listener.as_mut() {
                let gateway_poll: Poll = Poll::new()
                    .map_err(|_| Self::log_and_error(ErrorCode::IoErr, "failed to create Poll"))?;
                gateway_poll
                    .registry()
                    .register(gateway_listener, GATEWAY_LISTENER_TOKEN, Interest::READABLE)
                    .map_err(|_| {
                        Self::log_and_error(
                            ErrorCode::IoErr,
                            "failed to register gateway listener to poll",
                        )
                    })?;

                Some(gateway_poll)
            } else {
                None
            };

        // Accept one incoming connection from the gateway.
        let gateway_stream: Option<SocketStream> = if let Some(mut gateway_poll) = gateway_poll {
            if let Some(gateway_listener) = self.gateway_listener.as_mut() {
                Some(
                    gateway_listener
                        .accept_timeout(
                            &mut gateway_poll,
                            Duration::from_secs(config::syscomm::ACCEPT_TIMEOUT_SECS),
                        )
                        .map_err(|_| {
                            Self::log_and_error(
                                ErrorCode::IoErr,
                                "error accepting connection from gateway",
                            )
                        })?,
                )
            } else {
                None
            }
        } else {
            None
        };

        // Map keeping track of the worker threads associated the user VM.
        let mut worker_threads: VecDeque<WorkerThreadHandle> = VecDeque::new();

        // Handle around the user VM.
        let uvm_handle: UserVmHandle =
            UserVmHandle::new(DEFAULT_CONNECTION_ID, self.uvm_stream, gateway_stream);

        'main_loop: loop {
            let venv: Arc<Mutex<VirtualEnviromentDirectory>> = self.venv.clone();
            let assembler: Arc<Mutex<RequestAssembler>> = self.assembler.clone();

            user_vm_poll
                .poll(&mut events, None)
                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to poll user VM events"))?;

            for event in events.iter() {
                match event.token() {
                    // Process control-plane messages before anything else.
                    CONTROL_PLANE_TOKEN => {
                        let cmd: control_plane::Command =
                            match control_plane::try_read_command(&mut self.control_plane_stream) {
                                Ok(cmd) => cmd,
                                Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
                                Err(e) => {
                                    error!(
                                        "failed reading command from control-plane (error={e:?})"
                                    );
                                    return Err(Error::new(
                                        ErrorCode::IoErr,
                                        "failed reading command from control-plane",
                                    ));
                                },
                            };
                        match cmd {
                            control_plane::Command::Shutdown => {
                                info!("linuxd received shutdown message from control-plane");

                                // Close connection to user VM.
                                let uvm_handle: UserVmHandle = uvm_handle.clone();
                                info!("shutting down user VM");

                                Self::close_connection(
                                    uvm_handle,
                                    &user_vm_poll,
                                    Some(worker_threads),
                                );

                                break 'main_loop;
                            },
                        }
                    },

                    USER_VM_TOKEN => {
                        // Receive a message from the user virtual machine.
                        let uvm_handle: UserVmHandle = uvm_handle.clone();
                        let message: Message = match Self::recv(uvm_handle.get_user_vm_stream()) {
                            Ok(message) => message,

                            Err(error_kind) => match error_kind {
                                ErrorKind::WouldBlock => continue,
                                ErrorKind::UnexpectedEof | ErrorKind::ConnectionReset => {
                                    info!("connection from user VM closed");

                                    Self::close_connection(
                                        uvm_handle,
                                        &user_vm_poll,
                                        Some(worker_threads),
                                    );

                                    break 'main_loop;
                                },
                                _ => {
                                    let reason: String =
                                        format!("failed to read message (error={error_kind:?})");
                                    unimplemented!("handle: {reason}");
                                },
                            },
                        };

                        trace!(
                            "message.source={:?}, message.destination={:?}, message.type={:?}",
                            { message.source },
                            { message.destination },
                            message.message_type,
                        );

                        let source: ThreadIdentifier = match { message.source }.as_id() {
                            Err(tid) => tid,
                            Ok(pid) => {
                                unimplemented!(
                                    "received message from process {pid:?} instead of thread"
                                );
                            },
                        };

                        // Check if process is associated with a virtual environment.
                        let (channel_tx, channel_rx): (
                            Sender<VenvCommand>,
                            Option<Receiver<VenvCommand>>,
                        ) = {
                            let mut venv: MutexGuard<'_, VirtualEnviromentDirectory> =
                                venv.lock().unwrap();
                            let env = venv.get(source);
                            if let Some(env) = env {
                                (env.get_channel_tx(), None)
                            } else {
                                // Join a new virtual environment.
                                match venv.join(source, VirtualEnvironmentIdentifier::NEW) {
                                    Ok((_, channel_tx, channel_rx)) => {
                                        (channel_tx, Some(channel_rx))
                                    },
                                    Err(error) => {
                                        warn!(
                                            "failed to join new virtual environment \
                                             (error={error:?})"
                                        );
                                        let message: Message =
                                            crate::build_error(source, error.code);

                                        let uvm_stream: Arc<Mutex<(SocketStream, VecDeque<u8>)>> =
                                            uvm_handle.get_user_vm_stream();
                                        let mut guard: MutexGuard<
                                            '_,
                                            (SocketStream, VecDeque<u8>),
                                        > = match uvm_stream.lock() {
                                            Ok(guard) => guard,
                                            Err(e) => {
                                                error!(
                                                    "error acquiring lock on user VM stream \
                                                     (error={e:?})"
                                                );
                                                continue;
                                            },
                                        };
                                        let (locked_uvm_stream, _): &mut (
                                            SocketStream,
                                            VecDeque<u8>,
                                        ) = &mut guard;

                                        locked_uvm_stream.write_all(&message.to_bytes()).map_err(
                                            |_| {
                                                let reason = "failed to write to user VM stream";
                                                error!("{reason}");
                                                Error::new(ErrorCode::IoErr, reason)
                                            },
                                        )?;
                                        continue;
                                    },
                                }
                            }
                        };

                        // Spawn a new worker thread, if necessary.
                        if let Some(channel_rx) = channel_rx {
                            // Spawn a thread to handle the message.
                            let assembler = assembler.clone();

                            // Spawn an interruptible thread to handle the message.
                            let worker_thread_handle: WorkerThreadHandle =
                                WorkerThreadHandle::spawn(
                                    source,
                                    channel_rx,
                                    channel_tx.clone(),
                                    uvm_handle,
                                    assembler,
                                )?;
                            worker_threads.push_back(worker_thread_handle);
                        }

                        // Dispatch message to worker thread.
                        if let Err(error) = channel_tx.send(VenvCommand::Work(message)) {
                            error!(
                                "run(): failed to dispatch message to worker thread \
                                 (tid={source:?}, error={error:?})"
                            );
                            // Remove thread from the virtual environment.
                            let mut venv: MutexGuard<'_, VirtualEnviromentDirectory> =
                                venv.lock().unwrap();
                            if let Err(error) = venv.leave(source) {
                                warn!(
                                    "run(): failed to remove thread from virtual environment \
                                     (tid={source:?}, error={error:?})",
                                );
                            }
                        }
                    },

                    Token(t) => {
                        // This should never happen, but if it does we should ignore the spurious
                        // wake up and continue processing connections.
                        error!("poll received notification from unrecognised token {t:?}");
                        continue;
                    },
                }
            }
        }

        // TODO: https://github.com/nanvix/nanvix/issues/639
        Ok(())
    }

    /// Read a message from the user VM stream. We need to handle the situation where we can only
    /// do a partial read, so we keep a buffer alongside our socket. It is safe to have this buffer
    /// dynamically sized, as it will always be smaller than one message size.
    fn recv(uvm_stream: Arc<Mutex<(SocketStream, VecDeque<u8>)>>) -> Result<Message, ErrorKind> {
        let mut guard: MutexGuard<'_, (SocketStream, VecDeque<u8>)> = match uvm_stream.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("error acquiring lock on user VM stream (error={e:?})");
                return Err(ErrorKind::InvalidData);
            },
        };
        let (locked_uvm_stream, partial_read_buffer): &mut (SocketStream, VecDeque<u8>) =
            &mut guard;

        let mut buf: [u8; config::kernel::IPC_MESSAGE_SIZE] =
            [0u8; config::kernel::IPC_MESSAGE_SIZE];

        let mut num_filled = 0;
        if !partial_read_buffer.is_empty() {
            // Prepare data in buffer for partial read.
            partial_read_buffer.make_contiguous();
            let partial_bytes = partial_read_buffer.as_slices().0;

            // We take the minimum at the end just in case, but the partial read should always be
            // strictly smaller than the message size.
            let num_partial_read = partial_bytes.len().min(buf.len());

            buf[..num_partial_read].copy_from_slice(&partial_bytes[..num_partial_read]);

            // Clear partial read buffer.
            partial_read_buffer.clear();
            num_filled += num_partial_read;
        }
        // Post-condition: partial_read_buffer is empty.

        match locked_uvm_stream.try_read_exact(&mut buf[num_filled..]) {
            Ok(n) => {
                // Handle partial reads by copying all we have read to the partial read buffer and
                // returning a WouldBlock indicating that we need more data.
                if n + num_filled < buf.len() {
                    partial_read_buffer.extend(&buf[..(n + num_filled)]);
                    return Err(ErrorKind::WouldBlock);
                }
            },
            Err(e) => return Err(e.kind()),
        }

        let message: Message = match Message::try_from_bytes(buf) {
            Ok(message) => message,
            Err(e) => {
                let reason: String = format!("failed to parse message (error={e:?})");
                unimplemented!("handle: {}", reason);
            },
        };

        Ok(message)
    }
}
