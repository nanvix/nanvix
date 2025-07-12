// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod assemble;
mod gateway;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    build_error,
    dirent,
    fcntl,
    linuxd::gateway::{
        GatewayCommand,
        GatewayHandle,
        GatewayReactor,
    },
    message::{
        RequestAssembler,
        RequestAssemblerTrait,
    },
    poll,
    socket,
    times,
    unistd,
    venv::VirtualEnviromentDirectory,
};
use ::anyhow::Result;
use ::mio::{Events, Interest, Poll, Token};
use ::slab_external::Slab;
use ::std::{
    io::{
        ErrorKind,
        Read,
    },
    sync::{
        mpsc::{
            Receiver,
            Sender,
        },
        Arc,
        Mutex,
        MutexGuard,
    },
    thread::{
        self,
        ThreadId,
    },
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
use ::sysapi::{
    sys_types::c_ssize_t,
    unistd::{
        STDERR_FILENO,
        STDIN_FILENO,
        STDOUT_FILENO,
    },
};
use ::syscall::{
    dirent::message::GetDirectoryEntriesRequest,
    fcntl::message::{
        FileAdvisoryInformationRequest,
        FileControlRequest,
        FileSpaceControlRequest,
        OpenAtRequest,
        RenameAtRequest,
        UnlinkAtRequest,
    },
    message::LinuxDaemonMessagePart,
    sys::{
        socket::message::{
            AcceptSocketRequest,
            BindSocketRequest,
            ConnectSocketRequest,
            CreateSocketPairRequest,
            CreateSocketRequest,
            GetPeerNameRequest,
            GetSockNameRequest,
            ListenSocketRequest,
            ReceiveSocketRequest,
            SendSocketRequest,
            ShutdownSocketRequest,
        },
        stat::message::{
            FileChmodAtRequest,
            FileChmodRequest,
            FileStatAtRequest,
            FileStatRequest,
            MakeDirectoryAtRequest,
            UpdateFileAccessTimeAtRequest,
            UpdateFileAccessTimeRequest,
        },
        times::message::TimesRequest,
    },
    unistd::message::{
        ChangeDirectoryRequest,
        CloseRequest,
        CloseResponse,
        FileAccessAtRequest,
        FileChdirRequest,
        FileChownAtRequest,
        FileChownRequest,
        FileDataSyncRequest,
        FileSyncRequest,
        FileTruncateRequest,
        GetIdsRequest,
        LinkAtRequest,
        PartialReadRequest,
        PartialWriteRequest,
        PipeRequest,
        ReadLinkAtRequest,
        ReadRequest,
        ReadResponse,
        SeekRequest,
        SymbolicLinkAtRequest,
        WriteRequest,
        WriteResponse,
    },
    venv::VirtualEnvironmentIdentifier,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
    LINUXD,
};
use ::syscomm::{SocketListener, SocketStream};

//==================================================================================================
// Structures
//==================================================================================================

/// State associated with a user VM connected to this linuxd instance.
#[derive(Clone)]
struct UserVmHandle {
    user_vm_stream: Arc<Mutex<SocketStream>>,
    gw_stream: Option<Arc<Mutex<SocketStream>>>,
}

impl UserVmHandle {
    pub fn new(user_vm_stream: SocketStream, gw_stream: Option<SocketStream>) -> Result<Self> {
        Ok(Self {
            user_vm_stream: Arc::new(Mutex::new(user_vm_stream)),
            gw_stream: gw_stream.map(|stream| Arc::new(Mutex::new(stream))),
        })
    }

    pub fn get_user_vm_stream(&self) -> Arc<Mutex<SocketStream>> {
        self.user_vm_stream.clone()
    }

    pub fn get_gw_vm_stream(&self) -> Option<Arc<Mutex<SocketStream>>> {
        self.gw_stream.clone()
    }
}

pub struct LinuxDaemon {
    // The user VM or gateway listener sockets will not be accessed by different worker threads.
    // Instead, each one will have a SocketStream for the corresponding accepted connection.
    user_vm_listener: SocketListener,
    gateway_listener: Option<SocketListener>,
    assembler: Arc<Mutex<RequestAssembler>>,
    venv: Arc<Mutex<VirtualEnviromentDirectory>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    pub fn init(
        user_vm_listener: SocketListener,
        gateway_listener: Option<SocketListener>,
    ) -> Result<Self, Error> {
        Ok(Self {
            user_vm_listener,
            gateway_listener,
            assembler: Arc::new(Mutex::new(RequestAssembler::default())),
            venv: Arc::new(Mutex::new(VirtualEnviromentDirectory::new())),
        })
    }

    /*
    pub fn init_gateway_streams(
        &mut self,
        conn_id: usize
    ) -> Result<(GwStdinTx, GwStdoutTx), Error> {
        // Start the threads that will poll input from the gateway.
        // TODO: we could consider moving this to a design where a single polling thread manages
        // gateway connections from all active user VMs.
        let (gw_stdin_tx, gw_stdout_tx) = if let Some(gateway_listener) = &self.gateway_listener {
            // Accept the new gateway connection.
            let gateway_stream: SocketStream = loop {
                match user_vm_listener.accept() {
                    Ok(stream) => {
                        info!("Connected to user VM in: {:?}", stream.peer_addr());
                        break stream;
                    },
                    Err(error) => {
                        error!("Failed to accept connection: {error:?}");
                        continue;
                    },
                };
            };

            // For the STDIN channel senders (TX) need to wait for a response from the IO thread,
            // hence they send, together with the ReadRequest, the send endpoint of a channel where
            // they will wait for the response. For STDOUT senders need not to wait, hence no need
            // to also send the channel endpoint.
            let (gw_stdin_tx, gw_stdin_rx) = mpsc::channel::<(ReadRequest, Sender<Message>)>();
            let (gw_stdout_tx, gw_stdout_rx) = mpsc::channel::<WriteRequest>();

            // Make sure that the input stream from the gateway is set to blocking, as otherwise we
            // would not be able to differentiate between an EOF and a race between the application
            // code and the gateway.
            let mut gw_stdin_stream = gateway_stream
                .try_clone()
                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to clone stream"))?;
            gw_stdin_stream
                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to set non-blocing socket"))?;

            let _gw_stdin_thread: JoinHandle<Result<()>> = std::thread::spawn(move || {
                loop {
                    // Block waiting for the user VM to request reading from STDIN.
                    match gw_stdin_rx.recv() {
                        Ok((_read_request, response_tx)) => {
                            let mut response_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
                            let num_read = match gw_stdin_stream
                                .read(&mut response_buf) {
                                    Ok(n) => n,
                                    Err(e) => {
                                        let reason: String = format!("failed to read STDIN from gateway: {e:?}");
                                        error!("{}", reason);
                                        return Err(anyhow::anyhow!(reason));
                                    }
                                };
                            response_tx.send(ReadResponse::build(
                                0.into(),
                                num_read as c_ssize_t,
                                response_buf))?;
                        }
                        Err(RecvError) => {
                            info!("gateway STDIN channel disconnected");
                            break Ok(());
                        }
                    }
                }
            });

            let mut gw_stdout_stream = gateway_stream
                .try_clone()
                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to clone stream"))?;
            let _gw_stdout_thread: JoinHandle<Result<()>> = std::thread::spawn(move || {
                loop {
                    // Block waiting the user VM to request writing to stdout.
                    match gw_stdout_rx.recv() {
                        Ok(write_request) => {
                            gw_stdout_stream
                                .write_all(&write_request.buffer[..write_request.count as usize])?;

                            // We don't need to send anything in response of the write, as the
                            // writting thread has already moved on.
                        }
                        Err(RecvError) => {
                            let reason: String = "gateway STDOUT channel disconnected".to_string();
                            error!("{}", reason);
                            return Err(anyhow::anyhow!(reason));
                        }
                    }
                }
            });

            (Some(gw_stdin_tx), Some(gw_stdout_tx))
        } else {
            (None, None)
        }
    }
*/

    /// This helper method accepts connections into the main user VM listener socket, and, if
    /// necessary, accepts incoming connections for the gateway into this user VM.
    fn accept_connections(
        &mut self,
        user_vm_connections: &mut Slab<UserVmHandle>,
        poll: &Poll,
        start_token: usize,
        gw_handle: &Option<GatewayHandle>
    ) ->Result<(), Error> {
        // Accept new connection in a loop, as we have a non-blocking socket, and
        // we may have more than one connection pending to be accepted.
        loop {
            match self.user_vm_listener.accept() {
                Ok(mut user_vm_stream) => {
                    let entry = user_vm_connections.vacant_entry();
                    let token = Token(start_token + entry.key());

                    poll.registry().register(&mut user_vm_stream, token, Interest::READABLE | Interest::WRITABLE)
                        .map_err(|_| Error::new(ErrorCode::IoErr, "failed to register new user VM to poll"))?;

                    // Once we accept a connection from a user VM, trigger the gateway reactor to
                    // accept a command from the gateway.
                    // TODO: we should make this step more flexible to:
                    // 1. Avoid race conditions when multiple user VMs connect at the same time.
                    // 2. Support the situation where some user VMs may not have a gateway.
                    // 3. Support the situation where user VMs use a gateway lazily.
                    let gw_stream: Option<SocketStream> = if let Some(gw_handle) = gw_handle {
                        gw_handle.gw_cmd_tx
                            .send(GatewayCommand::AcceptConn)
                            .map_err(|_| Error::new(ErrorCode::IoErr, "failed to send message to GW thread"))?;
                        gw_handle.waker
                            .wake()
                            .map_err(|_| Error::new(ErrorCode::IoErr, "failed to wake GW thread"))?;
                        Some(gw_handle.gw_conn_rx
                            .recv()
                            .map_err(|_| Error::new(ErrorCode::IoErr, "failed to read response from GW thread"))?)
                    } else {
                        None
                    };

                    entry.insert(UserVmHandle::new(user_vm_stream, gw_stream)
                        .map_err(|_| Error::new(ErrorCode::IoErr, "failed to insert user VM handle to slab"))?);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connections to be accepted, break.
                    break;
                }
                Err(e) => {
                    // This is a fatal error for the user VM, but we don't want to
                    // kill linuxd.
                    error!("Error accepting connection from user VM: {e:?}");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Wrapper around the extraction of the connection from the slab, as we need a mutable
    /// reference, and with the function call we ensure we return the borrow.
    fn process_connection(
        &mut self,
        user_vm_connections: &mut Slab<UserVmHandle>,
        conn_id: usize
    ) ->Result<UserVmHandle, Error> {
        match user_vm_connections.get_mut(conn_id) {
            Some(user_vm_handle) => Ok(user_vm_handle.clone()),
            None => Err(Error::new(ErrorCode::InvalidArgument, "Invalid connection ID")),
        }
    }

    /// This is the main run loop for linuxd. It uses a listener socket to
    /// accept connections from multiple user VMs, and it polls over all
    /// active connections to serve requests.
    pub fn run(&mut self) -> Result<(), Error> {
        // We keep a slab of tokens to active connections from user VMs. We
        // reserve the first token for the main listener socket, that should
        // out-live user VMs, and a second token as a waker token for the
        // gateway thread.
        //
        // We use a slab because it is better suited to index a highly-dense
        // collection with usize keys.
        const LISTENER_TOKEN: Token = Token(0);
        const WAKER_TOKEN: Token = Token(1);
        const START_TOKEN: usize = 2;
        let mut user_vm_connections: Slab<UserVmHandle> = Slab::new();

        let mut poll = Poll::new().map_err(|_| Error::new(ErrorCode::IoErr, "failed to create Poll"))?;
        let mut events = Events::with_capacity(config::syscomm::MAX_NUM_POLL_EVENTS);
        poll
            .registry()
            .register(&mut self.user_vm_listener, LISTENER_TOKEN, Interest::READABLE)
            .map_err(|_| Error::new(ErrorCode::IoErr, "failed to register user VM listener to poll"))?;

        // Start a gateway reactor thread that polls the gateway listener socket for incoming
        // connections to user VMs.
        let gw_thread_handle: Option<GatewayHandle> = if let Some(gateway_listener) = self.gateway_listener {
            Some(GatewayReactor::spawn(gateway_listener, LISTENER_TOKEN, WAKER_TOKEN).map_err(|_| Error::new(ErrorCode::IoErr, "failed to spawn gateway reactor"))?)
        } else {
            None
        };

        loop {
            let venv: Arc<Mutex<VirtualEnviromentDirectory>> = self.venv.clone();
            let assembler: Arc<Mutex<RequestAssembler>> = self.assembler.clone();

            poll.poll(&mut events, None)
                .map_err(|_| Error::new(ErrorCode::IoErr, "failed to poll user VM events"))?;

            for event in events.iter() {
                match event.token() {
                    // First we see if we have received any messages on the main listener socket.
                    // These indicate new user VMs connecting to linuxd.
                    LISTENER_TOKEN => {
                        self.accept_connections(&mut user_vm_connections, &poll, START_TOKEN, &gw_thread_handle)?;
                    }

                    // Now we process events from active connections.
                    Token(t) => {
                        let conn_id = t - START_TOKEN;

                        // Receive a message from the user virtual machine.
                        let uvm_handle = self.process_connection(&mut user_vm_connections, conn_id)?;
                        let message: Message = match Self::recv(uvm_handle.get_user_vm_stream()) {
                            Ok(message) => message,

                            Err(error_kind) => match error_kind {
                                ErrorKind::WouldBlock => continue,
                                ErrorKind::UnexpectedEof => {
                                    info!("connection closed");

                                    let user_vm_stream = uvm_handle.get_user_vm_stream();
                                    let mut user_vm_stream = user_vm_stream
                                        .lock()
                                        .unwrap();
                                    poll.registry().deregister(&mut *user_vm_stream)
                                        .map_err(|_| Error::new(ErrorCode::IoErr, "failed to de-register user VM from poll"))?;

                                    user_vm_connections.remove(conn_id);
                                    continue;
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
                                unimplemented!("received message from process {pid:?} instead of thread");
                            },
                        };

                        // Check if process is associated with a virtual environment.
                        let (channel_tx, channel_rx): (Sender<Message>, Option<Receiver<Message>>) = {
                            let mut venv: MutexGuard<'_, VirtualEnviromentDirectory> = venv.lock().unwrap();
                            let env = venv.get(source);
                            if let Some(env) = env {
                                (env.get_channel_tx(), None)
                            } else {
                                // Join a new virtual environment.
                                match venv.join(source, VirtualEnvironmentIdentifier::NEW) {
                                    Ok((_, channel_tx, channel_rx)) => (channel_tx, Some(channel_rx)),
                                    Err(error) => {
                                        warn!("failed to join new virtual environment (error={error:?})");
                                        let message: Message = crate::build_error(source, error.code);
                                        Self::send(uvm_handle.get_user_vm_stream(), message).unwrap();
                                        continue;
                                    },
                                }
                            }
                        };

                        // Spawn a new worker thread, if necessary.
                        if let Some(channel_rx) = channel_rx {
                            // Spawn a thread to handle the message.
                            let venv: Arc<Mutex<VirtualEnviromentDirectory>> = venv.clone();
                            let assembler = assembler.clone();
                            let _ = std::thread::spawn(move || {
                                Self::handle_message(channel_rx, uvm_handle, venv, assembler);
                            });
                        }

                        // Dispatch message to worker thread.
                        if let Err(error) = channel_tx.send(message) {
                            error!(
                                "run(): failed to dispatch message to worker thread (tid={source:?}, \
                                 error={error:?})"
                            );
                            // Remove thread from the virtual environment.
                            let mut venv: MutexGuard<'_, VirtualEnviromentDirectory> = venv.lock().unwrap();
                            if let Err(error) = venv.leave(source) {
                                warn!(
                                    "run(): failed to remove thread from virtual environment (tid={source:?}, \
                                     error={error:?})",
                                );
                            }
                        }
                    }
                }
            }
        }

        // TODO: kill gateway thread

        // TODO: https://github.com/nanvix/nanvix/issues/639
        #[allow(unreachable_code)]
        Ok(())
    }

    fn handle_message(
        channel_rx: Receiver<Message>,
        uvm_handle: UserVmHandle,
        venv: Arc<Mutex<VirtualEnviromentDirectory>>,
        assembler: Arc<Mutex<RequestAssembler>>,
    ) {
        let worker_tid: ThreadId = thread::current().id();
        let uvm_stream = uvm_handle.get_user_vm_stream();
        let gw_stream = uvm_handle.get_gw_vm_stream();

        loop {
            let message: Message = match channel_rx.recv() {
                Ok(message) => message,
                Err(error) => {
                    error!(
                        "handle_message(): failed to receive message from channel, stopping \
                         (worker_tid={worker_tid:?}, error={error:?})"
                    );
                    break;
                },
            };
            let source: ThreadIdentifier = match { message.source }.as_id() {
                Err(tid) => tid,
                Ok(_) => {
                    unreachable!("messages that are in this channel always address threads");
                },
            };

            match message.message_type {
                sys::ipc::MessageType::Empty => panic!("received empty message"),
                sys::ipc::MessageType::Interrupt => panic!("received interrupt message"),
                sys::ipc::MessageType::Exception => panic!("received exception message"),
                sys::ipc::MessageType::Ipc => panic!("received IPC message"),
                sys::ipc::MessageType::ProcessTerminationEvent => {
                    panic!("received process termination event message")
                },
                sys::ipc::MessageType::Ikc => {
                    match LinuxDaemonMessage::try_from_bytes(message.payload) {
                        Ok(message) => {
                            let message: Message = match message.header {
                                // The system calls are interposed before being forwarded to the
                                // backend provider.
                                LinuxDaemonMessageHeader::CloseRequest
                                | LinuxDaemonMessageHeader::ReadRequest
                                | LinuxDaemonMessageHeader::WriteRequest => {
                                    Self::handle_special_messages(
                                        gw_stream.clone(),
                                        source,
                                        message,
                                    )
                                },

                                // The following system calls have their request and response
                                // data fit in a single message. Thus, they can be immediately
                                // forwarded to the backend provider.
                                LinuxDaemonMessageHeader::AcceptSocketRequest
                                | LinuxDaemonMessageHeader::BindSocketRequest
                                | LinuxDaemonMessageHeader::ConnectSocketRequest
                                | LinuxDaemonMessageHeader::CreateSocketPairRequest
                                | LinuxDaemonMessageHeader::CreateSocketRequest
                                | LinuxDaemonMessageHeader::FileAdvisoryInformationRequest
                                | LinuxDaemonMessageHeader::FileChdirRequest
                                | LinuxDaemonMessageHeader::FileChmodRequest
                                | LinuxDaemonMessageHeader::FileChownRequest
                                | LinuxDaemonMessageHeader::FileControlRequest
                                | LinuxDaemonMessageHeader::FileDataSyncRequest
                                | LinuxDaemonMessageHeader::FileSpaceControlRequest
                                | LinuxDaemonMessageHeader::FileSyncRequest
                                | LinuxDaemonMessageHeader::FileTruncateRequest
                                | LinuxDaemonMessageHeader::GetIdsRequest
                                | LinuxDaemonMessageHeader::GetPeerNameRequest
                                | LinuxDaemonMessageHeader::GetSockNameRequest
                                | LinuxDaemonMessageHeader::ListenSocketRequest
                                | LinuxDaemonMessageHeader::PartialReadRequest
                                | LinuxDaemonMessageHeader::PartialWriteRequest
                                | LinuxDaemonMessageHeader::ReceiveSocketRequest
                                | LinuxDaemonMessageHeader::SeekRequest
                                | LinuxDaemonMessageHeader::SendSocketRequest
                                | LinuxDaemonMessageHeader::ShutdownSocketRequest
                                | LinuxDaemonMessageHeader::TimesRequest
                                | LinuxDaemonMessageHeader::PipeRequest
                                | LinuxDaemonMessageHeader::PollRequest
                                | LinuxDaemonMessageHeader::UpdateFileAccessTimeRequest => {
                                    Self::handle_short_request_messages(source, message)
                                },

                                // The following system calls have their request data fit in a
                                // single message, but their response data is too large to fit in a
                                // single message. Thus, their response is split into multiple
                                // messages.
                                LinuxDaemonMessageHeader::FileStatRequest
                                | LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryRequest
                                | LinuxDaemonMessageHeader::GetDirectoryEntriesRequest => {
                                    Self::handle_long_response_messages(
                                        uvm_stream.clone(),
                                        source,
                                        message,
                                    );
                                    continue;
                                },

                                // The following system calls have request data that is too large to
                                // fit in a single message. Thus, their request is split into multiple
                                // messages.
                                LinuxDaemonMessageHeader::ChangeDirectoryRequestPart
                                | LinuxDaemonMessageHeader::FileStatAtRequestPart
                                | LinuxDaemonMessageHeader::FileAccessAtRequestPart
                                | LinuxDaemonMessageHeader::SymbolicLinkAtRequestPart
                                | LinuxDaemonMessageHeader::LinkAtRequestPart
                                | LinuxDaemonMessageHeader::ReadLinkAtRequestPart
                                | LinuxDaemonMessageHeader::MakeDirectoryAtRequestPart
                                | LinuxDaemonMessageHeader::UpdateFileAccessTimeAtRequestPart
                                | LinuxDaemonMessageHeader::FileChownAtRequestPart
                                | LinuxDaemonMessageHeader::FileChmodAtRequestPart
                                | LinuxDaemonMessageHeader::OpenAtRequestPart
                                | LinuxDaemonMessageHeader::RenameAtRequestPart
                                | LinuxDaemonMessageHeader::UnlinkAtRequestPart => {
                                    Self::handle_long_request_messages(
                                        uvm_stream.clone(),
                                        assembler.clone(),
                                        source,
                                        message,
                                    );
                                    continue;
                                },

                                _ => Self::do_error(source, ErrorCode::InvalidMessage),
                            };
                            Self::send(uvm_stream.clone(), message).unwrap();
                        },
                        Err(e) => {
                            error!("failed to parse Linux daemon message (error={e:?})");
                        },
                    }
                },
            }
        }
    }

    fn handle_special_messages(
        gw_stream: Option<Arc<Mutex<SocketStream>>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Message {
        match message.header {
            LinuxDaemonMessageHeader::CloseRequest => {
                let request: CloseRequest = CloseRequest::from_bytes(message.payload);
                Self::handle_close_request(source, request)
            },
            LinuxDaemonMessageHeader::ReadRequest => {
                let request: ReadRequest = ReadRequest::from_bytes(message.payload);
                Self::handle_read_request(gw_stream, source, request)
            },
            LinuxDaemonMessageHeader::WriteRequest => {
                let request: WriteRequest = WriteRequest::from_bytes(message.payload);
                Self::handle_write_request(gw_stream, source, request)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected special message {:?}", header)
            },
        }
    }

    fn handle_short_request_messages(
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Message {
        match message.header {
            LinuxDaemonMessageHeader::AcceptSocketRequest => {
                let request: AcceptSocketRequest = AcceptSocketRequest::from_bytes(message.payload);
                socket::do_accept(source, request)
            },
            LinuxDaemonMessageHeader::BindSocketRequest => {
                let request: BindSocketRequest = BindSocketRequest::from_bytes(message.payload);
                socket::do_bind(source, request)
            },
            LinuxDaemonMessageHeader::ConnectSocketRequest => {
                let request: ConnectSocketRequest =
                    ConnectSocketRequest::from_bytes(message.payload);
                socket::do_connect(source, request)
            },
            LinuxDaemonMessageHeader::CreateSocketPairRequest => {
                let request: CreateSocketPairRequest =
                    CreateSocketPairRequest::from_bytes(message.payload);
                socket::do_socketpair(source, request)
            },
            LinuxDaemonMessageHeader::CreateSocketRequest => {
                let request: CreateSocketRequest = CreateSocketRequest::from_bytes(message.payload);
                socket::do_socket(source, request)
            },
            LinuxDaemonMessageHeader::FileAdvisoryInformationRequest => {
                let request: FileAdvisoryInformationRequest =
                    FileAdvisoryInformationRequest::from_bytes(message.payload);
                fcntl::do_posix_fadvise(source, request)
            },
            LinuxDaemonMessageHeader::FileChdirRequest => {
                let request: FileChdirRequest = FileChdirRequest::from_bytes(message.payload);
                unistd::do_fchdir(source, request)
            },
            LinuxDaemonMessageHeader::FileChmodRequest => {
                let request: FileChmodRequest = FileChmodRequest::from_bytes(message.payload);
                fcntl::do_fchmod(source, request)
            },
            LinuxDaemonMessageHeader::FileChownRequest => {
                let request: FileChownRequest = FileChownRequest::from_bytes(message.payload);
                unistd::do_fchown(source, request)
            },
            LinuxDaemonMessageHeader::FileControlRequest => {
                let request: FileControlRequest = FileControlRequest::from_bytes(message.payload);
                fcntl::do_fcntl(source, request)
            },
            LinuxDaemonMessageHeader::FileDataSyncRequest => {
                let request: FileDataSyncRequest = FileDataSyncRequest::from_bytes(message.payload);
                unistd::do_fdatasync(source, request)
            },
            LinuxDaemonMessageHeader::FileSpaceControlRequest => {
                let request: FileSpaceControlRequest =
                    FileSpaceControlRequest::from_bytes(message.payload);
                fcntl::do_posix_fallocate(source, request)
            },
            LinuxDaemonMessageHeader::FileSyncRequest => {
                let request: FileSyncRequest = FileSyncRequest::from_bytes(message.payload);
                unistd::do_fsync(source, request)
            },
            LinuxDaemonMessageHeader::FileTruncateRequest => {
                let request: FileTruncateRequest = FileTruncateRequest::from_bytes(message.payload);
                unistd::do_ftruncate(source, request)
            },
            LinuxDaemonMessageHeader::GetIdsRequest => {
                let request: GetIdsRequest = GetIdsRequest::from_bytes(message.payload);
                unistd::do_getids(source, request)
            },
            LinuxDaemonMessageHeader::GetPeerNameRequest => {
                let request: GetPeerNameRequest = GetPeerNameRequest::from_bytes(message.payload);
                socket::do_getpeername(source, request)
            },
            LinuxDaemonMessageHeader::GetSockNameRequest => {
                let request: GetSockNameRequest = GetSockNameRequest::from_bytes(message.payload);
                socket::do_getsockname(source, request)
            },
            LinuxDaemonMessageHeader::ListenSocketRequest => {
                let request: ListenSocketRequest = ListenSocketRequest::from_bytes(message.payload);
                socket::do_listen(source, request)
            },
            LinuxDaemonMessageHeader::PartialReadRequest => {
                let request: PartialReadRequest = PartialReadRequest::from_bytes(message.payload);
                unistd::do_pread(source, request)
            },
            LinuxDaemonMessageHeader::PartialWriteRequest => {
                let request: PartialWriteRequest = PartialWriteRequest::from_bytes(message.payload);
                unistd::do_pwrite(source, request)
            },
            LinuxDaemonMessageHeader::PollRequest => {
                let request: syscall::poll::message::PollRequest =
                    syscall::poll::message::PollRequest::from_bytes(message.payload);
                poll::do_poll(source, request)
            },
            LinuxDaemonMessageHeader::ReceiveSocketRequest => {
                let request: ReceiveSocketRequest =
                    ReceiveSocketRequest::from_bytes(message.payload);
                socket::do_recv(source, request)
            },
            LinuxDaemonMessageHeader::SeekRequest => {
                let request: SeekRequest = SeekRequest::from_bytes(message.payload);
                unistd::do_lseek(source, request)
            },
            LinuxDaemonMessageHeader::SendSocketRequest => {
                let request: SendSocketRequest = SendSocketRequest::from_bytes(message.payload);
                socket::do_send(source, request)
            },
            LinuxDaemonMessageHeader::ShutdownSocketRequest => {
                let request: ShutdownSocketRequest =
                    ShutdownSocketRequest::from_bytes(message.payload);
                socket::do_shutdown(source, request)
            },
            LinuxDaemonMessageHeader::TimesRequest => {
                let request: TimesRequest = TimesRequest::from_bytes(message.payload);
                times::do_times(source, request)
            },
            LinuxDaemonMessageHeader::UpdateFileAccessTimeRequest => {
                let request: UpdateFileAccessTimeRequest =
                    UpdateFileAccessTimeRequest::from_bytes(message.payload);
                fcntl::do_futimens(source, request)
            },
            LinuxDaemonMessageHeader::PipeRequest => {
                let _request = PipeRequest::from_bytes(message.payload);
                unistd::do_pipe(source)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected short message {:?}", header)
            },
        }
    }

    fn handle_long_request_messages(
        uvm_stream: Arc<Mutex<SocketStream>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) {
        match message.header {
            LinuxDaemonMessageHeader::ChangeDirectoryRequestPart => {
                Self::handle_long_request::<ChangeDirectoryRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::FileAccessAtRequestPart => {
                Self::handle_long_request::<FileAccessAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::FileStatAtRequestPart => {
                Self::handle_long_request::<FileStatAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::SymbolicLinkAtRequestPart => {
                Self::handle_long_request::<SymbolicLinkAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::LinkAtRequestPart => {
                Self::handle_long_request::<LinkAtRequest>(uvm_stream, assembler, source, &message);
            },
            LinuxDaemonMessageHeader::ReadLinkAtRequestPart => {
                Self::handle_long_request::<ReadLinkAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::MakeDirectoryAtRequestPart => {
                Self::handle_long_request::<MakeDirectoryAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::UpdateFileAccessTimeAtRequestPart => {
                Self::handle_long_request::<UpdateFileAccessTimeAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::FileChownAtRequestPart => {
                Self::handle_long_request::<FileChownAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::FileChmodAtRequestPart => {
                Self::handle_long_request::<FileChmodAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::OpenAtRequestPart => {
                Self::handle_long_request::<OpenAtRequest>(uvm_stream, assembler, source, &message);
            },
            LinuxDaemonMessageHeader::RenameAtRequestPart => {
                Self::handle_long_request::<RenameAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            LinuxDaemonMessageHeader::UnlinkAtRequestPart => {
                Self::handle_long_request::<UnlinkAtRequest>(
                    uvm_stream, assembler, source, &message,
                );
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected long request message {:?}", header)
            },
        }
    }

    fn handle_long_response_messages(
        uvm_stream: Arc<Mutex<SocketStream>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) {
        match message.header {
            LinuxDaemonMessageHeader::FileStatRequest => {
                Self::handle_fstat_request(uvm_stream, source, message);
            },
            LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryRequest => {
                Self::handle_getcwd_request(uvm_stream, source);
            },
            LinuxDaemonMessageHeader::GetDirectoryEntriesRequest => {
                Self::handle_getdents_request(uvm_stream, source, message);
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected long response message {:?}", header)
            },
        }
    }

    // Read a message from the TCP stream.
    fn recv(uvm_stream: Arc<Mutex<SocketStream>>) -> Result<Message, ErrorKind> {
        let mut buf: [u8; config::kernel::IPC_MESSAGE_SIZE] =
            [0u8; config::kernel::IPC_MESSAGE_SIZE];

        let mut locked_uvm_stream: MutexGuard<'_, SocketStream> = uvm_stream.lock().unwrap();

        if let Err(e) = locked_uvm_stream.read_exact(&mut buf) {
            return Err(e.kind());
        };

        let message: Message = match Message::try_from_bytes(buf) {
            Ok(message) => message,
            Err(e) => {
                let reason: String = format!("failed to parse message (error={e:?})");
                unimplemented!("handle: {}", reason);
            },
        };

        Ok(message)
    }

    // Send a message to the TCP stream.
    fn send(uvm_stream: Arc<Mutex<SocketStream>>, message: Message) -> Result<()> {
        let bytes = message.to_bytes();

        loop {
            let mut locked_uvm_stream = uvm_stream.lock().unwrap();

            match locked_uvm_stream.write_all(&bytes) {
                Ok(_) => break Ok(()),
                Err(e) => {
                    match e.kind() {
                        ErrorKind::WouldBlock => {
                            // The stream is not ready to write, retry.
                            continue;
                        },
                        error_kind => {
                            unimplemented!(
                                "handle: failed to write message (error={error_kind:?})"
                            );
                        },
                    }
                },
            }
        }
    }

    fn do_error(source: ThreadIdentifier, code: ErrorCode) -> Message {
        Message::new(
            MessageSender::from(LINUXD),
            MessageReceiver::from(source),
            MessageType::Ikc,
            Some(code),
            [0u8; Message::PAYLOAD_SIZE],
        )
    }

    fn handle_close_request(source: ThreadIdentifier, request: CloseRequest) -> Message {
        // Inspect file descriptor that is being closed, as we need to
        // handle standard file descriptors specially.
        match request.fd {
            // Closing standard file descriptors.
            STDIN_FILENO | STDOUT_FILENO | STDERR_FILENO => {
                // Perform a fake close, as standard file descriptors
                // are shared with the current process.
                CloseResponse::build(source, 0)
            },
            // Closing other file descriptors.
            _ => unistd::do_close(source, request),
        }
    }

    fn handle_write_request(
        gw_stream: Option<Arc<Mutex<SocketStream>>>,
        source: ThreadIdentifier,
        mut request: WriteRequest,
    ) -> Message {
        trace!("handle_write_request(): source={source:?}, request={request:?}");
        // Check if writing to gateway.
        if request.fd == STDOUT_FILENO || request.fd == STDERR_FILENO {
            let gw_stream = if let Some(gw_stream) = gw_stream {
                gw_stream
            } else {
                error!("handle_write_request(): trying to write to stdout without a gateway configured");
                return build_error(source, ErrorCode::InvalidArgument);
            };

            // Check if write size is invalid.
            if request.count == 0 {
                // Writing zero-bytes to STDOUT is not allowed, as we used this to signal EOF.
                error!("handle_write_request(): trying to write zero bytes to STDOUT");
                build_error(source, ErrorCode::InvalidArgument)
            } else {
                profiler::timestamp_message!(&mut request.buffer, 0);
                let count: usize = request.count as usize;
                /*
                if let Err(error) = gateway_stdout_tx.send(request) {
                    debug!("failed to write buffer to the gateway (error={error:?})");
                    // TODO: Check error conversion.
                    return build_error(source, ErrorCode::ConnectionReset);
                }
                */
                if let Err(_) = gw_stream
                    .lock()
                    .unwrap()
                    .write_all(&request.buffer[..count]) {
                    error!("failed to write to gateway socket");
                    return build_error(source, ErrorCode::IoErr);
                }

                // We don't wait for the IO thread to confirm that the write was correct, as writes
                // are fully non-blocking.
                debug!("wrote {count} bytes to the gateway");
                WriteResponse::build(source, count as i32)
            }
        } else {
            // Write to other file descriptor.
            unistd::do_write(source, request)
        }
    }

    fn handle_read_request(
        gw_stream: Option<Arc<Mutex<SocketStream>>>,
        source: ThreadIdentifier,
        request: ReadRequest,
    ) -> Message {
        trace!("handle_read_request(): source={source:?}, request={request:?}");
        // Check if reading from gateway.
        if request.fd == STDIN_FILENO {
            let gw_stream = if let Some(gw_stream) = gw_stream {
                gw_stream
            } else {
                error!("handle_read_request(): process tried to read from stdin but no gateway found");
                return ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE]);
            };

            // Read from the gateway thread.
            {
                let mut gw_stream = gw_stream.lock().unwrap();
                loop {
                    let mut response_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
                    let response = match gw_stream.read(&mut response_buf) {
                        Ok(0) => {
                            error!(
                                "handle_read_request(): error receiving request response from gateway STDIN: EOF"
                            );
                            ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE])
                        },
                        Ok(n) => {
                            ReadResponse::build(
                                source,
                                n as c_ssize_t,
                                response_buf)
                        },
                        Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
                        _ => {
                            error!(
                                "handle_read_request(): error receiving request response from gateway STDIN");
                            ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE])
                        }
                    };

                    return response;
                }
            }
            /*
            if let Err(error) = gateway_stdin_tx.send((request, env.get_stdin_response_tx())) {
                error!(
                    "handle_read_request(): error sending request to gateway STDIN IO thread, returning EOF \
                    (error={error:?})"
                );
                return ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE]);
            }

            // Wait for response from IO thread.
            match env
                .get_stdin_response_rx()
                .recv() {
                    Ok(mut read_response) => {
                        // We don't have access to the source in the gateway IO thread, so we set
                        // it here.
                        read_response.destination = source.into();
                        read_response
                    },
                    Err(e) => {
                        error!(
                            "handle_read_request(): error receiving request response from gateway STDIN \
                            IO thread, returning EOF (error={e:?})"
                        );
                        ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE])
                    }
                }
            */
        } else {
            // Read from other file descriptor.
            unistd::do_read(source, request)
        }
    }

    fn handle_fstat_request(
        uvm_stream: Arc<Mutex<SocketStream>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) {
        let request: FileStatRequest = FileStatRequest::from_bytes(message.payload);
        let messages: Vec<Message> = fcntl::do_fstat(source, request);
        for message in messages {
            if let Err(e) = Self::send(uvm_stream.clone(), message) {
                error!("failed to send message (error={e:?})");
            }
        }
    }

    fn handle_getcwd_request(uvm_stream: Arc<Mutex<SocketStream>>, source: ThreadIdentifier) {
        let messages: Vec<Message> = unistd::do_getcwd(source);
        for message in messages {
            if let Err(e) = Self::send(uvm_stream.clone(), message) {
                error!("failed to send message (error={e:?})");
            }
        }
    }

    fn handle_getdents_request(
        uvm_stream: Arc<Mutex<SocketStream>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) {
        let request: GetDirectoryEntriesRequest =
            GetDirectoryEntriesRequest::from_bytes(message.payload);

        let messages: Vec<Message> = dirent::do_getdents(source, request);
        for message in messages {
            if let Err(e) = Self::send(uvm_stream.clone(), message) {
                error!("failed to send message (error={e:?})");
            }
        }
    }

    fn handle_long_request<T>(
        uvm_stream: Arc<Mutex<SocketStream>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        source: ThreadIdentifier,
        message: &LinuxDaemonMessage,
    ) where
        T: RequestAssemblerTrait,
    {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        trace!("handle_long_request(): source={source:?}, part={part:?}");

        let result: Result<Option<Vec<Message>>, Error> =
            assembler.lock().unwrap().process_message::<T>(source, part);

        match result {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = Self::send(uvm_stream.clone(), message) {
                        error!("failed to send message (error={e:?})");
                    }
                }
            },
            Ok(None) => {},
            Err(e) => {
                error!("failed to process request (error={e:?})");
                if let Err(e) = Self::send(uvm_stream.clone(), Self::do_error(source, e.code)) {
                    error!("failed to send error message (error={e:?})");
                }
            },
        }
    }
}
