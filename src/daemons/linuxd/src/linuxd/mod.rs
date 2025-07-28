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
    build_error,
    dirent,
    fcntl,
    message::{
        RequestAssembler,
        RequestAssemblerTrait,
    },
    poll,
    socket,
    times,
    unistd,
    venv::{
        VirtualEnviromentDirectory,
        VirtualEnvironment,
    },
};
use ::anyhow::Result;
use ::std::{
    io::{
        ErrorKind,
        Read,
    },
    sync::{
        mpsc::{
            Receiver,
            Sender,
            RecvError,
        },
        Arc,
        Mutex,
        MutexGuard,
        mpsc,
    },
    thread::{
        self,
        JoinHandle,
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
use ::syscomm::SocketStream;

//==================================================================================================
// Structures
//==================================================================================================

pub struct LinuxDaemon {
    assembler: Arc<Mutex<RequestAssembler>>,
    uvm_stream: Arc<Mutex<SocketStream>>,
    gateway_stream: Option<SocketStream>,
    venv: Arc<Mutex<VirtualEnviromentDirectory>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    pub fn init(
        uvm_stream: SocketStream,
        gateway_stream: Option<SocketStream>,
    ) -> Result<Self, Error> {
        if let Err(error) = uvm_stream.set_nonblocking(true) {
            let reason: &str = "failed to set UVM stream to non-blocking mode";
            error!("init(): {reason:?} (error={error:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        Ok(Self {
            assembler: Arc::new(Mutex::new(RequestAssembler::default())),
            uvm_stream: Arc::new(Mutex::new(uvm_stream)),
            gateway_stream,
            venv: Arc::new(Mutex::new(VirtualEnviromentDirectory::new())),
        })
    }

    pub fn run(&mut self) -> Result<(), Error> {
        // Start the thread that will poll input from the gateway.
        // TODO: when one linuxd instance manages more than one input stream we should encapsulate
        // this logic.
        let (gw_stdin_tx, gw_stdout_tx) = if let Some(gateway_stream) = &self.gateway_stream {
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
                .set_nonblocking(false)
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
                            // Label: linuxd::gw_stdin_thread::recv()
                            profiler::timestamp_message!(&mut response_buf, 0);
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
                        Ok(mut write_request) => {
                            // Label: linuxd::gw_stdout_thread::send()
                            profiler::timestamp_message!(&mut write_request.buffer, 0);
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
        };

        loop {
            let uvm_stream: Arc<Mutex<SocketStream>> = self.uvm_stream.clone();
            let venv: Arc<Mutex<VirtualEnviromentDirectory>> = self.venv.clone();
            let assembler: Arc<Mutex<RequestAssembler>> = self.assembler.clone();

            // Receive a message from the user virtual machine.
            let message: Message = match Self::recv(uvm_stream.clone()) {
                Ok(message) => message,

                Err(error_kind) => match error_kind {
                    ErrorKind::WouldBlock => continue,
                    ErrorKind::UnexpectedEof | ErrorKind::ConnectionReset => {
                        info!("connection closed");
                        break;
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
                            Self::send(uvm_stream.clone(), message).unwrap();
                            continue;
                        },
                    }
                }
            };

            // Spawn a new worker thread, if necessary.
            if let Some(channel_rx) = channel_rx {
                // Spawn a thread to handle the message.
                let venv: Arc<Mutex<VirtualEnviromentDirectory>> = venv.clone();
                let gw_stdin_tx = gw_stdin_tx.clone();
                let gw_stdout_tx = gw_stdout_tx.clone();
                let _ = std::thread::spawn(move || {
                    Self::handle_message(channel_rx, uvm_stream, gw_stdin_tx, gw_stdout_tx, venv, assembler);
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

        // TODO: https://github.com/nanvix/nanvix/issues/639
        Ok(())
    }

    fn handle_message(
        channel_rx: Receiver<Message>,
        uvm_stream: Arc<Mutex<SocketStream>>,
        gateway_stdin_tx: Option<Sender<(ReadRequest, Sender<Message>)>>,
        gateway_stdout_tx: Option<Sender<WriteRequest>>,
        venv: Arc<Mutex<VirtualEnviromentDirectory>>,
        assembler: Arc<Mutex<RequestAssembler>>,
    ) {
        let worker_tid: ThreadId = thread::current().id();

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
                                        gateway_stdin_tx.clone(),
                                        gateway_stdout_tx.clone(),
                                        venv.clone(),
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
        gateway_stdin_tx: Option<Sender<(ReadRequest, Sender<Message>)>>,
        gateway_stdout_tx: Option<Sender<WriteRequest>>,
        venv: Arc<Mutex<VirtualEnviromentDirectory>>,
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
                Self::handle_read_request(gateway_stdin_tx, venv, source, request)
            },
            LinuxDaemonMessageHeader::WriteRequest => {
                let request: WriteRequest = WriteRequest::from_bytes(message.payload);
                Self::handle_write_request(gateway_stdout_tx, source, request)
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
        gateway_stdout_tx: Option<Sender<WriteRequest>>,
        source: ThreadIdentifier,
        mut request: WriteRequest,
    ) -> Message {
        trace!("handle_write_request(): source={source:?}, request={request:?}");
        // Check if writing to gateway.
        if request.fd == STDOUT_FILENO || request.fd == STDERR_FILENO {
            let gateway_stdout_tx = if let Some(gateway_stdout_tx) = gateway_stdout_tx {
                gateway_stdout_tx
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
                // Label: linuxd::handle_write_request()
                profiler::timestamp_message!(&mut request.buffer, 0);
                let count: usize = request.count as usize;
                if let Err(error) = gateway_stdout_tx.send(request) {
                    debug!("failed to write buffer to the gateway (error={error:?})");
                    // TODO: Check error conversion.
                    return build_error(source, ErrorCode::ConnectionReset);
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
        gateway_stdin_tx: Option<Sender<(ReadRequest, Sender<Message>)>>,
        venv: Arc<Mutex<VirtualEnviromentDirectory>>,
        source: ThreadIdentifier,
        request: ReadRequest,
    ) -> Message {
        trace!("handle_read_request(): source={source:?}, request={request:?}");
        // Check if reading from gateway.
        if request.fd == STDIN_FILENO {
            let gateway_stdin_tx = if let Some(gateway_stdin_tx) = gateway_stdin_tx {
                gateway_stdin_tx
            } else {
                error!("handle_read_request(): process tried to read from stdin but no gateway found");
                return ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE]);
            };

            // Check if the process is associated with a virtual environment.
            let mut venv: MutexGuard<'_, VirtualEnviromentDirectory> = venv.lock().unwrap();
            let env: &mut VirtualEnvironment = if let Some(env) = venv.get_mut(source) {
                env
            } else {
                warn!(
                    "handle_read_request(): process is not associated with a virtual \
                     environment, returning EOF"
                );
                return ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE]);
            };

            // Send ReadRequest to gateway IO thread.
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

                        // Label: linuxd::handle_read_request()
                        profiler::timestamp_message!(&mut read_response.payload,
                            std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer));

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
