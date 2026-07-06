// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

extern crate alloc;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::{
        CONTROL_PLANE_CONNECT_TIMEOUT,
        READER_TASK_JOIN_TIMEOUT,
        WORKER_THREAD_SHUTDOWN_TIMEOUT,
    },
    message::RequestAssembler,
    syscalls::SyscallTable,
    user_vm_event::UserVmEvent,
    user_vm_handle::UserVmHandle,
    venv::{
        VenvCommand,
        VirtualEnviromentDirectory,
    },
    worker_thread::WorkerThreadHandle,
};
use ::anyhow::Result;
use ::config::kernel::IPC_MESSAGE_SIZE;
use ::control_plane_api::{
    self,
    ControlPlaneRegistrationMessage,
    NanvixdControlMessage,
};
use ::log::{
    debug,
    error,
    info,
    trace,
    warn,
};
use ::std::{
    collections::{
        HashMap,
        VecDeque,
    },
    io::ErrorKind,
    str::FromStr,
    sync::Arc,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        DataChunk,
        IkcFrame,
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::venv::VirtualEnvironmentIdentifier;
use ::syscomm::{
    ReadExact,
    SocketListener,
    SocketStream,
    SocketStreamReader,
    SocketStreamWriter,
    SocketType,
    UnboundSocket,
    WriteAll,
};
use ::tokio::{
    sync::{
        mpsc::{
            channel,
            Receiver,
            Sender,
        },
        Mutex,
        MutexGuard,
    },
    task::JoinHandle,
    time::timeout,
};
use ::user_vm_api::{
    self,
    UserVmIdentifier,
    NEW_USER_VM_MESSAGE_LEN,
};

//==================================================================================================
// Private Modules
//==================================================================================================

mod assemble;
mod error;
mod linux;
mod message;
mod time;
mod user_vm_event;
mod user_vm_handle;
mod venv;
mod worker_thread;

//==================================================================================================
// Public Modules
//==================================================================================================

pub mod args;
pub mod config;
pub mod syscalls;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Maximum number of messages that can be queued in a channel to a worker thread.
///
pub const WORKER_THREAD_CHANNEL_CAPACITY: usize = 1024;

///
/// # Description
///
/// Maximum number of messages that can be queued in a channel multiplexing messages from all user
/// VMs being served by this linuxd instance.
///
const USER_VM_CHANNEL_CAPACITY: usize = 1024;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Linux daemon that manages user VMs and handles system calls.
///
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. This is passed to system call handlers and
///   allows implementations to maintain context-specific information. Must implement `Sync + Send`.
///   Use `()` if no custom state is required.
///
pub struct LinuxDaemon<T: Sync + Send + 'static> {
    syscall_table: Arc<SyscallTable<T>>,
    assembler: Arc<Mutex<RequestAssembler>>,
    tenant_id: String,
    control_plane_sockaddr: String,
    control_plane_sockaddr_type: SocketType,
    user_vm_listener: SocketListener,
    venv: Arc<Mutex<VirtualEnviromentDirectory>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: Sync + Send + 'static> LinuxDaemon<T> {
    ///
    /// # Description
    ///
    /// Initializes a new Linux daemon.
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: System call table.
    /// - `tenant_id`: Unique tenant identifier.
    /// - `control_plane_sockaddr`: Control plane socket address.
    /// - `control_plane_sockaddr_type`: Control plane socket type.
    /// - `user_vm_listener`: User VM listener socket.
    ///
    /// # Returns
    ///
    /// Upon success, this function returns a new Linux daemon instance.
    /// Upon failure, an error is returned.
    ///
    pub fn init(
        syscall_table: Arc<SyscallTable<T>>,
        tenant_id: &str,
        control_plane_sockaddr: &str,
        control_plane_sockaddr_type: &str,
        user_vm_listener: SocketListener,
    ) -> Result<Self, Error> {
        let control_plane_sockaddr_type_parsed: SocketType =
            match SocketType::from_str(control_plane_sockaddr_type) {
                Ok(socket_type) => socket_type,
                Err(parse_error) => {
                    error!(
                        "failed to parse control-plane socket type (socket_type={}, \
                         error={parse_error:?})",
                        control_plane_sockaddr_type
                    );
                    return Err(Error::new(
                        ErrorCode::InvalidArgument,
                        "failed to parse control-plane socket type",
                    ));
                },
            };

        Ok(Self {
            syscall_table,
            assembler: Arc::new(Mutex::new(RequestAssembler::default())),
            tenant_id: tenant_id.to_string(),
            control_plane_sockaddr: control_plane_sockaddr.to_string(),
            control_plane_sockaddr_type: control_plane_sockaddr_type_parsed,
            user_vm_listener,
            venv: Arc::new(Mutex::new(VirtualEnviromentDirectory::new())),
        })
    }

    ///
    /// # Description
    ///
    /// This helper method establishes a connection with the control-plane.
    ///
    /// To connect to the control-plane, we first establish the raw connection, and then send our
    /// registration key (i.e. tenant_id) so that the control-plane can identify who is connecting.
    ///
    /// # Arguments
    ///
    /// `registration_key`: the key to send to the control-plane.
    ///
    /// # Returns
    ///
    /// The control-plane stream on success, an error otherwise.
    ///
    async fn accept_control_plane_connection(
        &self,
        registration_key: &str,
    ) -> Result<SocketStream, Error> {
        let unbound_socket: UnboundSocket = UnboundSocket::new(self.control_plane_sockaddr_type);
        match timeout(
            CONTROL_PLANE_CONNECT_TIMEOUT,
            unbound_socket.connect(&self.control_plane_sockaddr),
        )
        .await
        {
            Ok(Ok(mut socket)) => {
                let registration: Vec<u8> =
                    ControlPlaneRegistrationMessage::for_linuxd(registration_key)
                        .and_then(|msg| msg.to_bytes())
                        .map_err(|error| {
                            let reason: &str = "failed to encode control-plane registration";
                            error!(
                                "accept_control_plane_connection(): {reason} \
                                 (registration_key={}, error={error:?})",
                                registration_key
                            );
                            Error::new(ErrorCode::InvalidArgument, reason)
                        })?;
                socket.write_all(&registration).await.map_err(|error| {
                    let reason: &str = "failed to register control-plane connection";
                    error!(
                        "accept_control_plane_connection(): {reason} (registration_key={}, \
                         error={error:?})",
                        registration_key
                    );
                    Error::new(ErrorCode::ConnectionAborted, reason)
                })?;
                info!("Connected to control plane on: {:?}", self.control_plane_sockaddr);
                Ok(socket)
            },
            Ok(Err(error)) => {
                let reason: &str = "failed to connect to control-plane";
                error!(
                    "accept_control_plane_connection(): {reason} (addr={}, error={error:?})",
                    self.control_plane_sockaddr
                );
                Err(Error::new(ErrorCode::TryAgain, reason))
            },
            Err(_elapsed) => {
                let reason: &str = "timeout waiting to connect to control-plane";
                error!("accept_control_plane_connection(): {reason}");
                Err(Error::new(ErrorCode::OperationTimedOut, reason))
            },
        }
    }

    /// This helper method accepts connections into the main user VM listener socket, and, if
    /// necessary, accepts incoming connections for the gateway into this user VM.
    async fn accept_connections(
        &self,
        mut user_vm_stream: SocketStream,
        user_vm_event_tx: Sender<UserVmEvent>,
        control_plane_writer: Arc<Mutex<SocketStreamWriter>>,
    ) -> Result<(UserVmIdentifier, UserVmHandle), Error> {
        trace!("accepted connection from user VM (addr={user_vm_stream:?})",);

        let mut payload: [u8; NEW_USER_VM_MESSAGE_LEN] = [0u8; NEW_USER_VM_MESSAGE_LEN];
        user_vm_stream.read_exact(&mut payload).await.map_err(|e| {
            let reason: &'static str = "failed to read NewUserVm message from user VM";
            error!("{reason} (addr={user_vm_stream:?}, error={e:?})");
            Error::new(ErrorCode::IoErr, reason)
        })?;

        let new_msg: user_vm_api::NewUserVm = user_vm_api::NewUserVm::try_from_bytes(&payload)
            .map_err(|e| {
                let reason: &'static str = "failed to parse NewUserVm message from user VM";
                error!("{reason} (addr={user_vm_stream:?}, error={e:?})");
                Error::new(ErrorCode::InvalidArgument, reason)
            })?;
        let user_vm_id: UserVmIdentifier = new_msg.id();

        trace!("registered new user VM connection (vm_id={user_vm_id}, addr={user_vm_stream:?})",);

        // Spawn a background task that reads messages from this user VM and
        // enqueues them to the main reception channel monitored in this loop.
        let (user_vm_reader, user_vm_writer): (SocketStreamReader, SocketStreamWriter) =
            user_vm_stream.split();
        let user_vm_reader_handle: JoinHandle<Result<()>> =
            tokio::spawn(Self::user_vm_reader_loop(user_vm_id, user_vm_reader, user_vm_event_tx));

        let gateway_sockaddr: String = new_msg.gateway_sockaddr().to_string();

        // Gateway listener for this user VM.
        let user_vm_handle: UserVmHandle = UserVmHandle::new(
            user_vm_writer,
            u32::from(user_vm_id),
            &gateway_sockaddr,
            new_msg.gateway_socket_type(),
            user_vm_reader_handle,
            control_plane_writer,
        );

        {
            let gateway_user_vm_id: UserVmIdentifier = user_vm_id;
            let gateway_handle: UserVmHandle = user_vm_handle.clone();
            tokio::spawn(async move {
                trace!(
                    "accept_connections(): priming gateway listener (uvm_id={gateway_user_vm_id})"
                );
                if let Err(error) = gateway_handle.get_gateway_vm_stream().await {
                    error!(
                        "accept_connections(): failed to prime gateway listener \
                         (uvm_id={gateway_user_vm_id}, error={error:?})"
                    );
                } else {
                    trace!(
                        "accept_connections(): gateway listener primed \
                         (uvm_id={gateway_user_vm_id})"
                    );
                }
            });
        }

        Ok((user_vm_id, user_vm_handle))
    }

    ///
    /// # Description
    ///
    /// Helper method to close a connection to a user VM identified by the connection id. Closing
    /// the connection also involves stopping all associated worker threads.
    ///
    async fn close_connection(
        user_vm_handle: Option<UserVmHandle>,
        worker_threads: Option<VecDeque<WorkerThreadHandle>>,
    ) {
        // Join reader task in user VM handle.
        if let Some(user_vm_handle) = user_vm_handle {
            if let Some(mut user_vm_reader_handle) =
                user_vm_handle.take_user_vm_reader_handle().await
            {
                match timeout(READER_TASK_JOIN_TIMEOUT, &mut user_vm_reader_handle).await {
                    Ok(Ok(Ok(()))) => {
                        trace!("close_connection(): successfully joined user VM reader task");
                    },
                    Ok(Ok(Err(e))) => {
                        error!(
                            "close_connection(): user VM reader task returned error (error={e:?})"
                        );
                    },
                    Ok(Err(e)) => {
                        error!(
                            "close_connection(): error joining user VM reader task (error={e:?})"
                        );
                    },
                    Err(_elapsed) => {
                        warn!(
                            "close_connection(): timeout waiting for user VM reader task, \
                             aborting it"
                        );
                        user_vm_reader_handle.abort();
                    },
                }
            }
        } else {
            warn!("run(): harvesting user VM without a handle");
        }

        // Send a shutdown message to all worker threads associated
        // with this user VM.
        if let Some(mut worker_threads) = worker_threads {
            for mut worker_thread in worker_threads.drain(..) {
                trace!("sending interrupt to worker thread (thread_id={:?})", worker_thread.id);

                // Each worker thread may be in one of three states:
                // 1. Running
                // 2. Blocked on an async I/O operation (tokio socket read/write)
                // 3. Blocked on a synchronous libc syscall (read/write on non-gateway fds)
                // 4. Blocked waiting for a new message from the channel
                //
                // Calling stop() does two things:
                //   a) Triggers the cancellation watch channel, which immediately unblocks
                //      the worker if it is in an async I/O select! or a channel recv.
                //   b) Sends SIGUSR1 via pthread_kill, which causes any blocking libc
                //      syscall to return EINTR so the worker can exit.
                //
                // We also enqueue a Shutdown command as a belt-and-suspenders fallback so
                // the worker sees an explicit shutdown even if it is between select! rounds.
                //
                // Trigger cancellation first so a worker blocked on I/O or a syscall gets
                // unblocked and starts draining the channel before we attempt to enqueue
                // the shutdown command. This prevents a deadlock where the bounded channel
                // is full and the send suspends forever because the worker never drains it.
                //
                // If any of the commands fail, continue trying to drain the remaining
                // threads.
                if let Err(e) = worker_thread.stop() {
                    error!(
                        "error sending interrupt to worker thread (thread_id={:?}, error={e:?})",
                        worker_thread.id
                    );
                }
                match timeout(
                    WORKER_THREAD_SHUTDOWN_TIMEOUT,
                    worker_thread.cmd_tx.send(VenvCommand::Shutdown),
                )
                .await
                {
                    Ok(Ok(())) => {
                        trace!(
                            "close_connection(): sent shutdown command to worker thread \
                             (thread_id={:?})",
                            worker_thread.id
                        );
                    },
                    Ok(Err(e)) => {
                        error!(
                            "close_connection(): error sending shutdown command to worker thread \
                             (thread_id={:?}, error={e:?})",
                            worker_thread.id
                        );
                    },
                    Err(_elapsed) => {
                        warn!(
                            "close_connection(): timeout sending shutdown command to worker \
                             thread (thread_id={:?})",
                            worker_thread.id
                        );
                    },
                }
                match timeout(WORKER_THREAD_SHUTDOWN_TIMEOUT, &mut worker_thread.handle).await {
                    Ok(Ok(())) => {
                        trace!(
                            "close_connection(): successfully joined worker thread \
                             (thread_id={:?})",
                            worker_thread.id
                        );
                    },
                    Ok(Err(e)) => {
                        error!(
                            "close_connection(): error joining worker thread (thread_id={:?}, \
                             error={e:?})",
                            worker_thread.id
                        );
                    },
                    Err(_elapsed) => {
                        warn!(
                            "close_connection(): timeout waiting for worker thread, aborting it \
                             (thread_id={:?})",
                            worker_thread.id
                        );
                        worker_thread.handle.abort();
                    },
                }
            }
        }
    }

    ///
    /// # Description
    ///
    /// Spawns asynchronous cleanup for a user VM connection so the main event loop is not blocked
    /// by teardown of reader/worker tasks.
    ///
    fn spawn_connection_cleanup(
        user_vm_handle: Option<UserVmHandle>,
        worker_threads: Option<VecDeque<WorkerThreadHandle>>,
    ) {
        tokio::spawn(async move {
            Self::close_connection(user_vm_handle, worker_threads).await;
        });
    }

    fn log_and_error(code: ErrorCode, msg: &'static str) -> Error {
        error!("{msg}");
        Error::new(code, msg)
    }

    ///
    /// # Description
    ///
    /// Runs the Linux daemon main loop.
    ///
    /// # Returns
    ///
    /// Upon success, this function returns `Ok(())`.
    /// Upon failure, an error is returned.
    ///
    pub async fn run(self) -> Result<(), Error> {
        // Structure keeping track of the active user VM connections, indexed by their connection
        // ID. We use a slab to easily get the smallest available entry.
        let mut user_vm_connections: HashMap<UserVmIdentifier, UserVmHandle> = HashMap::new();

        // This queue is used to fan-in messages from all user VMs into a single queue that we can
        // monitor and then fan-out to the corresponding worker threads.
        let (user_vm_event_tx, mut user_vm_event_rx): (Sender<UserVmEvent>, Receiver<UserVmEvent>) =
            channel::<UserVmEvent>(USER_VM_CHANNEL_CAPACITY);

        // Map keeping track of the worker threads associated to each user VM identified by
        // connection ID. We use a HashMap and not a Slab because we need to support insert/removal
        // by key.
        let worker_threads: Arc<Mutex<HashMap<UserVmIdentifier, VecDeque<WorkerThreadHandle>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Accept the control-plane stream before entering the main loop.
        let control_plane_stream: SocketStream = self
            .accept_control_plane_connection(&self.tenant_id)
            .await?;

        // Split the control-plane stream so that the reader stays in the main loop and the writer
        // can be shared with gateway priming tasks to send GatewayReady notifications.
        let (mut control_plane_reader, control_plane_writer): (
            SocketStreamReader,
            SocketStreamWriter,
        ) = control_plane_stream.split();
        let control_plane_writer: Arc<Mutex<SocketStreamWriter>> =
            Arc::new(Mutex::new(control_plane_writer));

        let mut control_plane_buffer: [u8; NanvixdControlMessage::WIRE_SIZE] =
            [0u8; NanvixdControlMessage::WIRE_SIZE];
        let mut control_plane_buffer_filled: usize = 0;

        'main_loop: loop {
            tokio::select! {

                result = control_plane_reader.read(&mut control_plane_buffer[control_plane_buffer_filled..]) => {
                    match result {
                        Ok(0) => {
                            // Control-plane disconnected.
                            break 'main_loop;
                        },
                        Ok(n) => {
                            control_plane_buffer_filled += n;

                            if control_plane_buffer_filled == control_plane_buffer.len() {

                                let msg: NanvixdControlMessage = NanvixdControlMessage::try_from_bytes(&control_plane_buffer).map_err(|e| {
                                    let reason: &'static str = "failed parsing command from control-plane";
                                    error!("run(): {reason} (error={e:?})");
                                    Error::new(ErrorCode::IoErr, reason)
                                })?;


                                match msg.cmd() {
                                    control_plane_api::NanvixdCommand::Shutdown => {
                                        info!("linuxd received shutdown message from control-plane");
                                        let mut locked_worker_threads: MutexGuard<'_, HashMap<UserVmIdentifier, VecDeque<WorkerThreadHandle>>> = worker_threads.lock().await;
                                        for (uvm_id, handle) in user_vm_connections.drain() {
                                            info!("shutting down user VM (vm_id={uvm_id})");
                                            Self::close_connection(Some(handle), locked_worker_threads.remove(&uvm_id)).await;
                                        }
                                        if !locked_worker_threads.is_empty() {
                                            error!("finished shutdown with orphaned worker threads (conn_ids={:?})", locked_worker_threads.keys().collect::<Vec<_>>());
                                        }
                                        break 'main_loop;
                                    },
                                    // The following branch is for boilerplate purposes only.
                                    #[allow(unreachable_patterns)]
                                    _ => {
                                        control_plane_buffer.fill(0);
                                        control_plane_buffer_filled = 0;
                                        error!("received unexpected command from control-plane (cmd={:?})", msg.cmd());
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            let reason: &'static str = "failed reading command from control-plane";
                            error!("run(): {reason} (error={e:?})");
                            return Err(Error::new(ErrorCode::IoErr, reason));
                        },
                    };
                },

                // Accept at most one new user VM connection per loop.
                result = self.user_vm_listener.accept() => {
                    match result {
                        Ok(user_vm_stream) => {
                            let (user_vm_id, user_vm_handle): (UserVmIdentifier, UserVmHandle) =
                                self.accept_connections(user_vm_stream, user_vm_event_tx.clone(), control_plane_writer.clone()).await?;
                            user_vm_connections.insert(user_vm_id, user_vm_handle.clone());
                        },
                        Err(e) => {
                            let reason: &'static str = "error accepting connection from user VM";
                            error!("run(): {reason} (error={e:?})");
                            return Err(Self::log_and_error(ErrorCode::IoErr, reason));
                        },
                    }

                }

                // Process one message from one user VM.
                user_vm_event = user_vm_event_rx.recv() => {
                    let Some(event) = user_vm_event else {
                        // We are in this situation if all senders have dropped. This may happen if
                        // all user VMs are done executing.
                        trace!("run(): user_vm_rx closed, no more VM events");
                        continue 'main_loop;
                    };

                    match event {
                        UserVmEvent::Transfer { uvm_id, transfer } => {
                            let Some(uvm_handle) = user_vm_connections.get(&uvm_id).cloned() else {
                                warn!(
                                    "run(): received transfer for unknown VM (uvm_id={uvm_id}), ignoring"
                                );
                                continue 'main_loop;
                            };

                            match transfer {
                                IkcFrame::Message(message) => {
                                    if let Err(e) = self.forward_user_vm_msg_to_worker_thread(
                                            uvm_id,
                                            uvm_handle,
                                            message,
                                            worker_threads.clone(),
                                        )
                                        .await
                                    {
                                        let reason: &'static str = "error processing message from user VM, terminating it";
                                        error!("run(): {reason} (uvm_id={uvm_id}, error={e:?})");

                                        // Shutdown faulty user VM.
                                        Self::spawn_connection_cleanup(
                                            user_vm_connections.remove(&uvm_id),
                                            worker_threads.lock().await.remove(&uvm_id),
                                        );

                                        continue 'main_loop;
                                    }
                                },
                                IkcFrame::Bulk(bulk) => {
                                    // Route data chunk transfer to the appropriate worker thread using
                                    // the source thread identifier from the header.
                                    let source_tid: ThreadIdentifier = bulk.header().source_tid();
                                    trace!(
                                        "run(): routing data chunk transfer to worker thread \
                                         (uvm_id={uvm_id}, source_tid={source_tid:?}, \
                                         data_len={})",
                                        bulk.header().data_len(),
                                    );

                                    let venv_dir: Arc<Mutex<VirtualEnviromentDirectory>> =
                                        self.venv.clone();
                                    let channel_tx: Option<Sender<VenvCommand>> = {
                                        let guard: MutexGuard<'_, VirtualEnviromentDirectory> =
                                            venv_dir.lock().await;
                                        guard
                                            .get(uvm_id, source_tid)
                                            .map(|env| env.get_channel_tx())
                                    };

                                    if let Some(tx) = channel_tx {
                                        if let Err(error) =
                                            tx.send(VenvCommand::BulkData(bulk)).await
                                        {
                                            error!(
                                                "run(): failed to dispatch data chunk transfer to \
                                                 worker thread (uvm_id={uvm_id}, \
                                                 source_tid={source_tid:?}, error={error:?})"
                                            );
                                        }
                                    } else {
                                        warn!(
                                            "run(): no worker thread found for data chunk transfer \
                                             (uvm_id={uvm_id}, source_tid={source_tid:?})"
                                        );
                                    }
                                },
                            }
                        },

                        UserVmEvent::ConnectionClosed { uvm_id }
                        | UserVmEvent::ConnectionError { uvm_id, .. } => {
                            let kind_str: String = match &event {
                                UserVmEvent::ConnectionError { kind, .. } => format!("{kind:?}"),
                                _ => "Closed".to_string(),
                            };
                            debug!("run(): harvesting connection to user VM (uvm_id={uvm_id}, reason={kind_str})");

                            // Shutdown finished user VM.
                            Self::spawn_connection_cleanup(
                                user_vm_connections.remove(&uvm_id),
                                worker_threads.lock().await.remove(&uvm_id),
                            );
                        },
                    }
                },
            }
        }

        info!("linuxd disconnected");
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Read a transfer (message or bulk) from the user VM stream. The stream uses a framing
    /// protocol: a single frame-type byte precedes each frame.
    ///
    async fn recv(uvm_reader: &mut SocketStreamReader) -> Result<IkcFrame, ErrorKind> {
        // Read the frame type byte. An EOF here means the connection was closed cleanly.
        let mut frame_type: [u8; 1] = [0u8; 1];
        match uvm_reader.read_exact(&mut frame_type).await {
            Ok(_) => {},
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                return Err(ErrorKind::ConnectionAborted);
            },
            Err(e) => return Err(e.kind()),
        }

        match frame_type[0] {
            // Standard IPC message.
            IkcFrame::MESSAGE_FRAME => {
                let mut buf: [u8; IPC_MESSAGE_SIZE] = [0u8; IPC_MESSAGE_SIZE];
                uvm_reader
                    .read_exact(&mut buf)
                    .await
                    .map_err(|e| e.kind())?;

                let message: Message = match Message::try_from_bytes(buf) {
                    Ok(message) => message,
                    Err(e) => {
                        error!("recv(): failed to parse message (error={e:?})");
                        return Err(ErrorKind::InvalidData);
                    },
                };

                Ok(IkcFrame::Message(message))
            },
            // Data chunk transfer.
            IkcFrame::DATA_CHUNK_FRAME => {
                // Read the 4-byte little-endian length prefix.
                let mut len_buf: [u8; 4] = [0u8; 4];
                uvm_reader
                    .read_exact(&mut len_buf)
                    .await
                    .map_err(|e| e.kind())?;
                let payload_len: usize = u32::from_le_bytes(len_buf) as usize;

                // Read the full data chunk transfer payload (header + data).
                let mut payload: Vec<u8> = vec![0u8; payload_len];
                uvm_reader
                    .read_exact(&mut payload)
                    .await
                    .map_err(|e| e.kind())?;

                let bulk: DataChunk = DataChunk::try_from_bytes(&payload).map_err(|e| {
                    error!("recv(): failed to parse data chunk transfer (error={e:?})");
                    ErrorKind::InvalidData
                })?;

                Ok(IkcFrame::Bulk(bulk))
            },
            unknown => {
                error!("recv(): unknown frame type (type={unknown:#04x})");
                Err(ErrorKind::InvalidData)
            },
        }
    }
}

//==================================================================================================
// Internal Helpers (Tokio-based message processing)
//==================================================================================================

impl<T: Sync + Send + 'static> LinuxDaemon<T> {
    ///
    /// # Description
    ///
    /// This function implements the background asynchronous reading loop for a single user VM.
    /// Each user VM spawns a task like this, and all messages are fanned in to a single MPSC
    /// channel monitored by the main select!.
    ///
    async fn user_vm_reader_loop(
        uvm_id: UserVmIdentifier,
        mut uvm_reader: SocketStreamReader,
        uvm_events_tx: Sender<UserVmEvent>,
    ) -> Result<()> {
        trace!("user_vm_reader_loop(): starting (uvm_id={uvm_id})");

        loop {
            match Self::recv(&mut uvm_reader).await {
                Ok(transfer) => {
                    match &transfer {
                        IkcFrame::Message(message) => {
                            trace!(
                                "uservm.id={uvm_id}, message.source={:?}, \
                                 message.destination={:?}, message.type={:?}",
                                { message.source },
                                { message.destination },
                                message.message_type,
                            );
                        },
                        IkcFrame::Bulk(bulk) => {
                            trace!(
                                "uservm.id={uvm_id}, bulk.source_pid={:?}, \
                                 bulk.destination_pid={:?}, bulk.data_len={}",
                                bulk.header().source_pid(),
                                bulk.header().destination_pid(),
                                bulk.header().data_len(),
                            );
                        },
                    }

                    if uvm_events_tx
                        .send(UserVmEvent::Transfer { uvm_id, transfer })
                        .await
                        .is_err()
                    {
                        // If the receiver side is gone, just exit.
                        debug!(
                            "user_vm_reader_loop(): dispatcher dropped receiver (uvm_id={uvm_id})"
                        );
                        break;
                    }
                },
                Err(ErrorKind::ConnectionAborted) => {
                    // User VM closed connection.
                    trace!("user_vm_reader_loop(): connection aborted by peer (uvm_id={uvm_id})");

                    // Ignore return value, as we are breaking anyway.
                    let _ = uvm_events_tx
                        .send(UserVmEvent::ConnectionClosed { uvm_id })
                        .await;
                    break;
                },
                Err(kind) => {
                    error!("user_vm_reader_loop(): reader error (uvm_id={uvm_id}, error={kind:?})");
                    // Propagate the error to the main loop so that we can clean-up this user VM's
                    // state. Ignore send's return value as we are breaking from the loop anyway.
                    let _ = uvm_events_tx
                        .send(UserVmEvent::ConnectionError { uvm_id, kind })
                        .await;
                    break;
                },
            }
        }

        trace!("user_vm_reader_loop(): exiting (uvm_id={uvm_id})");

        Ok(())
    }

    ///
    /// # Description
    ///
    /// This function forwards a new message from a user VM to its corresponding worker thread. If
    /// no worker threads exist, a new one will be spawned.
    ///
    async fn forward_user_vm_msg_to_worker_thread(
        &self,
        uvm_id: UserVmIdentifier,
        uvm_handle: UserVmHandle,
        message: Message,
        worker_threads: Arc<Mutex<HashMap<UserVmIdentifier, VecDeque<WorkerThreadHandle>>>>,
    ) -> Result<(), Error> {
        let assembler: Arc<Mutex<RequestAssembler>> = self.assembler.clone();
        let venv_dir: Arc<Mutex<VirtualEnviromentDirectory>> = self.venv.clone();

        // The kernel stamps the originating thread into `message.source.tid`; a guest IKC request
        // always names a concrete thread. A `NONE` sentinel names no specific thread, so reject it
        // rather than keying virtual-environment state under the sentinel and misrouting work.
        let source: ThreadIdentifier = { message.source }.tid;
        if source.is_none() {
            let reason: &str = "received message with no originating thread";
            error!("forward_user_vm_msg_to_worker_thread(): {reason} (uvm_id={uvm_id})");
            return Err(Error::new(ErrorCode::InvalidMessage, reason));
        }

        // Ensure virtual environment association.
        let (channel_tx, channel_rx): (Sender<VenvCommand>, Option<Receiver<VenvCommand>>) = {
            let mut guard: MutexGuard<'_, VirtualEnviromentDirectory> = venv_dir.lock().await;
            if let Some(env) = guard.get(uvm_id, source) {
                (env.get_channel_tx(), None)
            } else {
                match guard.join(uvm_id, source, VirtualEnvironmentIdentifier::NEW) {
                    Ok((_, channel_tx, channel_rx)) => (channel_tx, Some(channel_rx)),
                    Err(error) => {
                        warn!("failed to join new virtual environment (error={error:?})");
                        let err_msg: Message = crate::build_error(source, error.code);
                        let uvm_writer: Arc<Mutex<SocketStreamWriter>> =
                            uvm_handle.get_user_vm_writer();
                        let mut writer_guard: MutexGuard<'_, SocketStreamWriter> =
                            uvm_writer.lock().await;
                        let err_bytes: [u8; IPC_MESSAGE_SIZE] = err_msg.to_bytes();
                        // Coalesce frame type byte and message payload into a single write
                        // to reduce syscall overhead and avoid sending the frame byte as a
                        // separate tiny segment.
                        let mut buf: [u8; 1 + IPC_MESSAGE_SIZE] = [0; 1 + IPC_MESSAGE_SIZE];
                        buf[0] = IkcFrame::MESSAGE_FRAME;
                        buf[1..].copy_from_slice(&err_bytes);
                        writer_guard.write_all(&buf).await.map_err(|e| {
                            let reason: &str = "failed to send error message to user VM";
                            error!(
                                "forward_user_vm_msg_to_worker_thread(): {reason} (error={e:?})"
                            );
                            Error::new(ErrorCode::IoErr, reason)
                        })?;

                        return Ok(());
                    },
                }
            }
        };

        if let Some(channel_rx) = channel_rx {
            let worker_thread_handle: WorkerThreadHandle = WorkerThreadHandle::spawn_worker_thread(
                source,
                channel_rx,
                channel_tx.clone(),
                uvm_handle.clone(),
                assembler.clone(),
                self.syscall_table.clone(),
            )?;
            worker_threads
                .lock()
                .await
                .entry(uvm_id)
                .or_default()
                .push_back(worker_thread_handle);
        }

        if let Err(error) = channel_tx.send(VenvCommand::Work(message)).await {
            error!(
                "run(): failed to dispatch message to worker thread (tid={source:?}, \
                 error={error:?})"
            );
            let mut guard = venv_dir.lock().await;
            if let Err(e) = guard.leave(uvm_id, source) {
                warn!(
                    "run(): failed to remove thread from virtual environment (uvm_id={uvm_id}, \
                     tid={source:?}, error={e:?})"
                );
            }
        }
        Ok(())
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds an error response message.
///
/// # Parameters
///
/// - `tid`: Thread identifier.
/// - `error`: Error code.
///
/// # Returns
///
/// A message with the error response.
///
pub fn build_error(tid: ThreadIdentifier, error: ErrorCode) -> Message {
    Message::new(
        MessageSender::new(::syscall::LINUXD, ThreadIdentifier::NONE),
        MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
        MessageType::Ikc,
        Some(error),
        [0u8; Message::PAYLOAD_SIZE],
    )
}
