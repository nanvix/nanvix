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
    config::restore_gate_sockaddr_builder,
    message::RequestAssembler,
    user_vm_handle::UserVmHandle,
    venv::{
        VenvCommand,
        VirtualEnviromentDirectory,
        VirtualEnvironment,
    },
    worker_thread::WorkerThreadHandle,
};
use ::anyhow::Result;
use ::control_plane_api;
use ::mio::{
    Events,
    Interest,
    Poll,
    Token,
};
use ::std::{
    collections::{
        HashMap,
        VecDeque,
    },
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
    BlockingSocketStream,
    Socket,
    SocketListener,
    SocketStream,
    SocketType,
};
use ::syslog::{
    error,
    info,
    trace,
    warn,
};
use ::user_vm_api::{
    self,
    RawUserVmIdentifier,
};

//==================================================================================================
// Constants
//==================================================================================================

/// We use ID 0 for the control-plane socket in the main poll structure.
const CONTROL_PLANE_CONNECTION_ID: usize = 0;
/// We use ID 1 for the listener socket accepting user VM connections.
const USER_VM_LISTENER_CONNECTION_ID: usize = 1;
/// We use ID 0 for the gateway listener socket in the gateway poll structure. Given that this
/// socket is monitored in a different poll, we can reuse the connection ID 0.
const GATEWAY_LISTENER_CONNECTION_ID: usize = 0;

//==================================================================================================
// Structures
//==================================================================================================

pub struct LinuxDaemon {
    assembler: Arc<Mutex<RequestAssembler>>,
    control_plane_sockaddr: String,
    control_plane_socktype: SocketType,
    user_vm_listener: SocketListener,
    venv: Arc<Mutex<VirtualEnviromentDirectory>>,
    in_l2: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    pub fn init(
        control_plane_sockaddr: String,
        control_plane_socktype: SocketType,
        user_vm_listener: SocketListener,
        in_l2: bool,
    ) -> Result<Self, Error> {
        Ok(Self {
            assembler: Arc::new(Mutex::new(RequestAssembler::default())),
            control_plane_sockaddr,
            control_plane_socktype,
            user_vm_listener,
            venv: Arc::new(Mutex::new(VirtualEnviromentDirectory::new())),
            in_l2,
        })
    }

    /// This helper method accepts a connection from the control-plane.
    fn accept_control_plane_connection(&self) -> Result<SocketStream> {
        match SocketStream::connect(
            self.control_plane_socktype,
            self.control_plane_sockaddr.clone(),
        ) {
            Ok(socket) => {
                info!("Connected to control plane on: {:?}", self.control_plane_sockaddr);
                Ok(socket)
            },
            Err(e) => {
                let reason: String = format!(
                    "failed to connect to control-plane socket address (address={}, error={e:?})",
                    self.control_plane_sockaddr.clone()
                );
                error!("{reason}");
                Err(anyhow::anyhow!(reason))
            },
        }
    }

    /// This helper method will trap waiting for a message in a given port if we are about to be
    /// snapshotted.
    fn trap_if_pending_snapshot(&self) -> Result<()> {
        if self.in_l2 {
            // We only need one token to block in the trap poll.
            const TRAP_TOKEN: Token = Token(0);

            let mut trap_listener: SocketListener =
                Socket::bind(SocketType::Tcp, restore_gate_sockaddr_builder())?;

            let mut trap_poll: Poll = Poll::new()?;
            trap_poll
                .registry()
                .register(&mut trap_listener, TRAP_TOKEN, Interest::READABLE)?;

            let mut events: Events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);
            'poll_loop: loop {
                // Deliberately print to stdout so that it can be captured by the snapshot creation
                // script.
                println!("{}", config::linuxd::SNAPSHOT_MAGIC_STRING);

                // Must poll infinitely. Timeout-based approaches will not work with the restore
                // process.
                trap_poll.poll(&mut events, None)?;
                for event in &events {
                    if event.token() != TRAP_TOKEN {
                        continue 'poll_loop;
                    }

                    if event.is_error() || event.is_read_closed() || event.is_write_closed() {
                        continue 'poll_loop;
                    }

                    if event.is_readable() {
                        if let Ok(_stream) = trap_listener.accept() {
                            break 'poll_loop;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// This helper method accepts connections into the main user VM listener socket, and, if
    /// necessary, accepts incoming connections for the gateway into this user VM.
    fn accept_connections(
        &mut self,
        user_vm_connections: &mut HashMap<RawUserVmIdentifier, UserVmHandle>,
        user_vm_poll: &Poll,
    ) -> Result<(), Error> {
        // Accept new connection in a loop, as we have a non-blocking socket, and
        // we may have more than one connection pending to be accepted.
        loop {
            match self.user_vm_listener.accept() {
                Ok(user_vm_stream) => {
                    // Temporarily set the user VM stream to blocking mode in order to receive the
                    // handshake message with metadata from the user VM.
                    let mut blocking_user_vm_stream: BlockingSocketStream =
                        user_vm_stream.set_blocking().map_err(|_| {
                            Self::log_and_error(
                                ErrorCode::IoErr,
                                "error setting stream to blocking mode",
                            )
                        })?;
                    let new_msg: user_vm_api::NewUserVm = user_vm_api::NewUserVm::recv(
                        &mut blocking_user_vm_stream,
                    )
                    .map_err(|_| {
                        Self::log_and_error(ErrorCode::IoErr, "error receiving ID from new user VM")
                    })?;
                    let mut user_vm_stream: SocketStream =
                        blocking_user_vm_stream.set_nonblocking().map_err(|_| {
                            Self::log_and_error(
                                ErrorCode::IoErr,
                                "error setting stream to non-blocking mode",
                            )
                        })?;
                    let user_vm_id: RawUserVmIdentifier = new_msg.id();

                    let token: Token = Token(user_vm_id as usize);
                    trace!("accepted connection from user VM (vm_id={user_vm_id})");

                    user_vm_poll
                        .registry()
                        .register(&mut user_vm_stream, token, Interest::READABLE)
                        .map_err(|_| {
                            Self::log_and_error(
                                ErrorCode::IoErr,
                                "failed to register new user VM to poll",
                            )
                        })?;

                    // After accepting a connection from the user VM, open a listening socket for
                    // the user VM's gateway.
                    let gateway_sockaddr: String = new_msg.gateway_sockaddr().to_string();
                    let gateway_socket_type: SocketType = new_msg.gateway_socket_type();
                    let mut gateway_listener: SocketListener =
                        match Socket::bind(gateway_socket_type, gateway_sockaddr.clone()) {
                            Ok(listener) => listener,
                            Err(e) => {
                                let reason: &'static str =
                                    "failed to bind gateway socket for user VM";
                                error!("{reason} (addr={gateway_sockaddr}, error={e:?})");
                                return Err(Self::log_and_error(ErrorCode::IoErr, reason));
                            },
                        };
                    trace!(
                        "linuxd started gateway listener for user VM (vm_id={user_vm_id}, \
                         addr={gateway_sockaddr})"
                    );

                    // Accept one connection. We use an ephemeral poll, but this step will become
                    // unnecessary when we introduce support for lazily accepting a gateway
                    // connection.
                    let mut gateway_poll: Poll = Poll::new().map_err(|_| {
                        Self::log_and_error(ErrorCode::IoErr, "failed to create Poll")
                    })?;
                    gateway_poll
                        .registry()
                        .register(
                            &mut gateway_listener,
                            Token(GATEWAY_LISTENER_CONNECTION_ID),
                            Interest::READABLE,
                        )
                        .map_err(|_| {
                            Self::log_and_error(
                                ErrorCode::IoErr,
                                "failed to register gateway listener to poll",
                            )
                        })?;

                    // Accept a connection once from nanvixd, and discard it. This lets nanvixd
                    // know, reliably, that the gateway is ready to accept connections and it can
                    // return its address to users without risk of race conditions.
                    gateway_listener
                        .accept_timeout(
                            &mut gateway_poll,
                            Duration::from_secs(config::syscomm::ACCEPT_TIMEOUT_SECS),
                        )
                        .map_err(|_| {
                            Self::log_and_error(
                                ErrorCode::IoErr,
                                "error accepting throw-away gateway connection from nanvixd",
                            )
                        })?;
                    trace!("linuxd accepted throw-away gateway connection from nanvixd");

                    // Now accept the real gateway connection. Once we move to lazily initialized
                    // connection, the next bit of logic will be moved elsewhere.
                    let gateway_stream: Option<SocketStream> = Some(
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
                    );

                    trace!(
                        "registered user VM handle (vm_id={user_vm_id}, gw_stream={})",
                        gateway_stream.is_some()
                    );
                    user_vm_connections
                        .insert(user_vm_id, UserVmHandle::new(user_vm_stream, gateway_stream));
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connections to be accepted, break.
                    break;
                },
                Err(e) => {
                    // This is a fatal error for the user VM, but we don't want to
                    // kill linuxd.
                    error!("Error accepting connection from user VM: {e:?}");
                    break;
                },
            }
        }

        Ok(())
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
        const USER_VM_LISTENER_TOKEN: Token = Token(USER_VM_LISTENER_CONNECTION_ID);

        let mut events: Events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);

        // Poll structure monitoring the user VM and control-plane connections.
        let mut user_vm_poll: Poll = Poll::new()
            .map_err(|_| Self::log_and_error(ErrorCode::IoErr, "failed to create Poll"))?;
        user_vm_poll
            .registry()
            .register(&mut self.user_vm_listener, USER_VM_LISTENER_TOKEN, Interest::READABLE)
            .map_err(|_| {
                Self::log_and_error(ErrorCode::IoErr, "failed to register user VM listener to poll")
            })?;

        // Structure keeping track of the active user VM connections, indexed by their connection
        // ID. We use a slab to easily get the smallest available entry.
        let mut user_vm_connections: HashMap<RawUserVmIdentifier, UserVmHandle> = HashMap::new();

        // Map keeping track of the worker threads associated to each user VM identified by
        // connection ID. We use a HashMap and not a Slab because we need to support insert/removal
        // by key.
        let mut worker_threads: HashMap<RawUserVmIdentifier, VecDeque<WorkerThreadHandle>> =
            HashMap::new();

        // Right before entering the main loop, block if we are pending to be snapshotted, and
        // accept the control-plane stream afterwards. We are pending to be snapshotted if we are
        // in this line of code, and are deployed in an L2 VM.
        self.trap_if_pending_snapshot().map_err(|_| {
            Self::log_and_error(ErrorCode::IoErr, "error conditionally trapping on snapshot gate")
        })?;
        let mut control_plane_stream: SocketStream =
            self.accept_control_plane_connection().map_err(|_| {
                Self::log_and_error(ErrorCode::IoErr, "failed to accept control-plane connection")
            })?;
        user_vm_poll
            .registry()
            .register(&mut control_plane_stream, CONTROL_PLANE_TOKEN, Interest::READABLE)
            .map_err(|_| {
                Self::log_and_error(ErrorCode::IoErr, "failed to register control-plane to poll")
            })?;

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
                        let cmd: control_plane_api::Command =
                            match control_plane_api::try_read_command(&mut control_plane_stream) {
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
                            control_plane_api::Command::Shutdown => {
                                info!("linuxd received shutdown message from control-plane");

                                // Close all existing connections to user VMs.
                                for (uvm_id, uvm_handle) in user_vm_connections.drain() {
                                    info!("shutting down user VM (vm_id={uvm_id})");

                                    Self::close_connection(
                                        uvm_handle,
                                        &user_vm_poll,
                                        worker_threads.remove(&uvm_id),
                                    );
                                }

                                // Draining the user VM connections should also drain the worker
                                // threads. Print an error if not.
                                if !worker_threads.is_empty() {
                                    error!(
                                        "finished shutdown with orphaned worker threads \
                                         (conn_ids={:?})",
                                        worker_threads.keys().collect::<Vec<_>>()
                                    );
                                }

                                break 'main_loop;
                            },
                        }
                    },

                    // Check if we have received any messages on the main listener socket.
                    // These indicate new user VMs connecting to linuxd.
                    USER_VM_LISTENER_TOKEN => {
                        self.accept_connections(&mut user_vm_connections, &user_vm_poll)?;
                    },

                    // Now we process events from active connections.
                    Token(t) => {
                        let uvm_id: RawUserVmIdentifier = match RawUserVmIdentifier::try_from(t) {
                            Ok(id) => id,
                            Err(e) => {
                                // Skip to next token.
                                error!("error clipping connection id (error={e:})");
                                continue;
                            },
                        };

                        let uvm_handle: UserVmHandle = match user_vm_connections.get(&uvm_id) {
                            Some(handle) => handle.clone(),
                            None => {
                                error!("error getting user VM handle (vm_id={uvm_id})");
                                continue;
                            },
                        };

                        // Drain the user VM stream until we cannot read any more messages (i.e.
                        // WouldBlock). This is because messages may be buffered in the poll.
                        'drain_loop: loop {
                            let uvm_handle: UserVmHandle = uvm_handle.clone();
                            let message: Message = match Self::recv(uvm_handle.get_user_vm_stream())
                            {
                                Ok(message) => message,

                                Err(error_kind) => match error_kind {
                                    // No more messages to read.
                                    ErrorKind::WouldBlock => break 'drain_loop,
                                    ErrorKind::UnexpectedEof | ErrorKind::ConnectionReset => {
                                        info!("connection from user VM closed (conn_id={uvm_id})");

                                        Self::close_connection(
                                            uvm_handle,
                                            &user_vm_poll,
                                            worker_threads.remove(&uvm_id),
                                        );
                                        user_vm_connections.remove(&uvm_id);

                                        break 'drain_loop;
                                    },
                                    _ => {
                                        let reason: String = format!(
                                            "failed to read message (error={error_kind:?})"
                                        );
                                        unimplemented!("handle: {reason}");
                                    },
                                },
                            };

                            trace!(
                                "uservm.id={uvm_id}, message.source={:?}, \
                                 message.destination={:?}, message.type={:?}",
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
                                let env: Option<&VirtualEnvironment> = venv.get(uvm_id, source);
                                if let Some(env) = env {
                                    (env.get_channel_tx(), None)
                                } else {
                                    // Join a new virtual environment.
                                    match venv.join(
                                        uvm_id,
                                        source,
                                        VirtualEnvironmentIdentifier::NEW,
                                    ) {
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

                                            let uvm_stream: Arc<
                                                Mutex<(SocketStream, VecDeque<u8>)>,
                                            > = uvm_handle.get_user_vm_stream();
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
                                                    break 'drain_loop;
                                                },
                                            };
                                            let (locked_uvm_stream, _): &mut (
                                                SocketStream,
                                                VecDeque<u8>,
                                            ) = &mut guard;

                                            locked_uvm_stream
                                                .write_all(&message.to_bytes())
                                                .map_err(|_| {
                                                    let reason =
                                                        "failed to write to user VM stream";
                                                    error!("{reason}");
                                                    Error::new(ErrorCode::IoErr, reason)
                                                })?;
                                            break 'drain_loop;
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
                                worker_threads
                                    .entry(uvm_id)
                                    .or_default()
                                    .push_back(worker_thread_handle);
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
                                if let Err(error) = venv.leave(uvm_id, source) {
                                    warn!(
                                        "run(): failed to remove thread from virtual environment \
                                         (uvm_id={uvm_id}, tid={source:?}, error={error:?})",
                                    );
                                }
                            }
                        }
                    },
                }
            }
        }

        info!("linuxd disconnected");
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
