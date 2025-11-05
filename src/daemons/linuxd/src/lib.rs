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
        restore_gate_sockaddr_builder,
        CONTROL_PLANE_CONNECT_TIMEOUT,
        READER_TASK_JOIN_TIMEOUT,
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
use ::config::{
    kernel::IPC_MESSAGE_SIZE,
    linuxd::SNAPSHOT_MAGIC_STRING,
};
use ::control_plane_api::{
    self,
    NanvixdControlMessage,
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
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
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
use ::syslog::{
    debug,
    error,
    info,
    trace,
    warn,
};
use ::tokio::{
    net::TcpListener,
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
    control_plane_sockaddr: String,
    control_plane_sockaddr_type: SocketType,
    user_vm_listener: SocketListener,
    venv: Arc<Mutex<VirtualEnviromentDirectory>>,
    in_l2: bool,
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
    /// - `control_plane_sockaddr`: Control plane socket address.
    /// - `control_plane_sockaddr_type`: Control plane socket type.
    /// - `user_vm_listener`: User VM listener socket.
    /// - `in_l2`: Whether the daemon is running in an L2 VM.
    ///
    /// # Returns
    ///
    /// Upon success, this function returns a new Linux daemon instance.
    /// Upon failure, an error is returned.
    ///
    pub fn init(
        syscall_table: Arc<SyscallTable<T>>,
        control_plane_sockaddr: &str,
        control_plane_sockaddr_type: &str,
        user_vm_listener: SocketListener,
        in_l2: bool,
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
            control_plane_sockaddr: control_plane_sockaddr.to_string(),
            control_plane_sockaddr_type: control_plane_sockaddr_type_parsed,
            user_vm_listener,
            venv: Arc::new(Mutex::new(VirtualEnviromentDirectory::new())),
            in_l2,
        })
    }

    /// This helper method accepts a connection from the control-plane.
    async fn accept_control_plane_connection(&self) -> Result<SocketStream, Error> {
        // The control-plane socket type depends on whether we are deploying linuxd in
        // an L2 VM or not.

        let unbound_socket: UnboundSocket = UnboundSocket::new(self.control_plane_sockaddr_type);
        match timeout(
            CONTROL_PLANE_CONNECT_TIMEOUT,
            unbound_socket.connect(&self.control_plane_sockaddr),
        )
        .await
        {
            Ok(Ok(socket)) => {
                info!("Connected to control plane on: {:?}", self.control_plane_sockaddr);
                Ok(socket)
            },
            Ok(Err(_error)) => {
                let reason: &str = "failed to connect to control-plane";
                error!("accept_control_plane_connection(): {reason}");
                Err(Error::new(ErrorCode::TryAgain, reason))
            },
            Err(_elapsed) => {
                let reason: &str = "timeout waiting to connect to control-plane";
                error!("accept_control_plane_connection(): {reason}");
                Err(Error::new(ErrorCode::OperationTimedOut, reason))
            },
        }
    }

    /// This helper method will trap waiting for a message in a given port if we are about to be
    /// snapshotted.
    async fn trap_if_pending_snapshot(&self) -> Result<()> {
        if self.in_l2 {
            let trap_listener: TcpListener =
                TcpListener::bind(restore_gate_sockaddr_builder()).await?;

            // Deliberately print to stdout so that it can be captured by the snapshot creation
            // script.
            println!("{}", SNAPSHOT_MAGIC_STRING);

            trap_listener.accept().await?;
        }

        Ok(())
    }

    /// This helper method accepts connections into the main user VM listener socket, and, if
    /// necessary, accepts incoming connections for the gateway into this user VM.
    async fn accept_connections(
        &self,
        mut user_vm_stream: SocketStream,
        user_vm_event_tx: Sender<UserVmEvent>,
    ) -> Result<(UserVmIdentifier, UserVmHandle), Error> {
        trace!("accepted connection from user VM (addr={user_vm_stream:?})",);

        debug!("waiting for user vm information");

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

        // Gateway listener for this user VM.
        Ok((
            user_vm_id,
            UserVmHandle::new(
                user_vm_writer,
                new_msg.gateway_sockaddr(),
                new_msg.gateway_socket_type(),
                user_vm_reader_handle,
            ),
        ))
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
                if let Err(e) = worker_thread.cmd_tx.send(VenvCommand::Shutdown).await {
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
                if let Err(e) = worker_thread.handle.await {
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

        // Right before entering the main loop, block if we are pending to be snapshotted, and
        // accept the control-plane stream afterwards. We are pending to be snapshotted if we are
        // in this line of code, and are deployed in an L2 VM.
        self.trap_if_pending_snapshot().await.map_err(|_| {
            Self::log_and_error(ErrorCode::IoErr, "error conditionally trapping on snapshot gate")
        })?;
        let mut control_plane_stream: SocketStream = self.accept_control_plane_connection().await?;

        let mut control_plane_buffer: [u8; ::std::mem::size_of::<NanvixdControlMessage>()] =
            [0u8; ::std::mem::size_of::<NanvixdControlMessage>()];
        let mut control_plane_buffer_filled: usize = 0;

        'main_loop: loop {
            tokio::select! {

                result = control_plane_stream.read(&mut control_plane_buffer[control_plane_buffer_filled..]) => {
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
                                self.accept_connections(user_vm_stream, user_vm_event_tx.clone()).await?;
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
                        UserVmEvent::Message { uvm_id, message } => {
                            let Some(uvm_handle) = user_vm_connections.get(&uvm_id).cloned() else {
                                warn!(
                                    "run(): received message for unknown VM (uvm_id={uvm_id}), ignoring"
                                );
                                continue 'main_loop;
                            };

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
                                Self::close_connection(
                                    user_vm_connections.remove(&uvm_id),
                                    worker_threads.lock().await.remove(&uvm_id),
                                ).await;

                                continue 'main_loop;
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
                            Self::close_connection(
                                user_vm_connections.remove(&uvm_id),
                                worker_threads.lock().await.remove(&uvm_id),
                            ).await;
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
    /// Read a message from the user VM stream.
    ///
    async fn recv(uvm_reader: &mut SocketStreamReader) -> Result<Message, ErrorKind> {
        let mut buf: [u8; IPC_MESSAGE_SIZE] = [0u8; IPC_MESSAGE_SIZE];
        uvm_reader
            .read_exact(&mut buf)
            .await
            .map_err(|e| e.kind())?;

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
                Ok(message) => {
                    trace!(
                        "uservm.id={uvm_id}, message.source={:?}, message.destination={:?}, \
                         message.type={:?}",
                        { message.source },
                        { message.destination },
                        message.message_type,
                    );

                    if uvm_events_tx
                        .send(UserVmEvent::Message { uvm_id, message })
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

        let source: ThreadIdentifier = match { message.source }.as_id() {
            Err(tid) => tid,
            Ok(pid) => {
                unimplemented!("received message from process {pid:?} instead of thread");
            },
        };

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
                        writer_guard.write_all(&err_bytes).await.map_err(|e| {
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
        MessageSender::from(::syscall::LINUXD),
        MessageReceiver::from(tid),
        MessageType::Ikc,
        Some(error),
        [0u8; Message::PAYLOAD_SIZE],
    )
}
