// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use crate::{
    build_error,
    dirent,
    error::WorkerThreadError,
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
        VenvCommand,
        VirtualEnvironment,
        VirtualEnviromentDirectory,
    },
};
use libc::{
    sigaction,
    sigemptyset,
    pthread_kill,
    pthread_self,
    SIGUSR1,
    c_int,
};
use ::std::{
    io::ErrorKind,
    mem,
    ptr,
    sync::{
        atomic::{
            AtomicUsize,
            Ordering
        },
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
        JoinHandle,
        ThreadId,
    }
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
use ::sysapi::unistd::{
    STDERR_FILENO,
    STDIN_FILENO,
    STDOUT_FILENO,
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
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
    LINUXD,
};
use ::syscomm::SocketStream;

const INTERRUPT_SIGNAL: c_int = SIGUSR1;

/// State associated with a worker thread in linuxd.
pub struct WorkerThreadHandle {
    // Internal thread identifier in linuxd.
    pub id: ThreadIdentifier,
    // Underlying tid returned by pthread.
    pub pthread_id: Arc<AtomicUsize>,
    pub handle: JoinHandle<()>,
    // Handle to send shutdown messages to message queue.
    pub cmd_tx: Sender<VenvCommand>,
}

/// Our signal handler is a no-op that will just interrupt any blocking system calls, and make the
/// thread error and return EINTR.
extern "C" fn linuxd_worker_thread_signal_handler(_: i32) {}

impl WorkerThreadHandle {
    fn install_signal_handler() {
        // SAFETY: we install a signal handler that is a no-op so this is safe.
        let ret = unsafe {
            let sig_action = sigaction {
                sa_sigaction: linuxd_worker_thread_signal_handler as usize,
                // Empty set to not block any other signals that may happen during signal handling.
                sa_mask: {
                    let mut set = mem::zeroed();
                    sigemptyset(&mut set);
                    set
                },
                // No SA_RESTART so that syscall will return EINTR.
                sa_flags: 0,
                sa_restorer: None,
            };

            sigaction(INTERRUPT_SIGNAL, &sig_action, ptr::null_mut())
        };

        if ret != 0 {
            // Notify the error, but don't fail.
            let errno: i32 = unsafe { *libc::__errno_location() };
            error!("error installing signal handler (errno={errno:?})");
        }
    }

    /// Spawn an interruptible worker thread.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        id: ThreadIdentifier,
        channel_rx: Receiver<VenvCommand>,
        channel_tx: Sender<VenvCommand>,
        uvm_stream: Arc<Mutex<SocketStream>>,
        gw_stdin_tx: Option<Sender<(ReadRequest, Sender<Message>)>>,
        gw_stdout_tx: Option<Sender<WriteRequest>>,
        venv: Arc<Mutex<VirtualEnviromentDirectory>>,
        assembler: Arc<Mutex<RequestAssembler>>,
    ) ->Result<Self, Error> {
        // We use an atomic to pass the id of the created thread back to the caller context. We
        // need this because std::thread's JoinHandle does not expose the tid.
        let pthread_id_holder = Arc::new(AtomicUsize::new(0));

        // Make copies to return as part of the thread handle.
        let pthread_id_holder_clone = pthread_id_holder.clone();

        let join_handle = thread::spawn(move || {
            Self::install_signal_handler();

            let pthread_id = unsafe { pthread_self() };
            pthread_id_holder.store(pthread_id as usize, Ordering::Relaxed);

            Self::handle_message(channel_rx, uvm_stream, gw_stdin_tx, gw_stdout_tx, venv, assembler);

            trace!("thread shutting down after receiving interrupt (pthread_id={pthread_id})");
        });

        Ok(Self {
            id,
            pthread_id: pthread_id_holder_clone,
            handle: join_handle,
            cmd_tx: channel_tx,
        })
    }

    /// Stop a worker-thread by sending an interrupt.
    pub fn stop(&self) -> Result<(), Error> {
        let raw_tid = self.pthread_id.load(Ordering::Relaxed);

        if raw_tid == 0 {
            let reason = "trying to stop thread with tid 0";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // SAFETY: we call pthread_kill on a non-zero TID after we have managed to send a message
        // to its reception queue, so the thread is alive and safe.
        let pthread_id = raw_tid as libc::pthread_t;
        unsafe { pthread_kill(pthread_id, INTERRUPT_SIGNAL) };

        Ok(())
    }

    fn handle_message(
        channel_rx: Receiver<VenvCommand>,
        uvm_stream: Arc<Mutex<SocketStream>>,
        gateway_stdin_tx: Option<Sender<(ReadRequest, Sender<Message>)>>,
        gateway_stdout_tx: Option<Sender<WriteRequest>>,
        venv: Arc<Mutex<VirtualEnviromentDirectory>>,
        assembler: Arc<Mutex<RequestAssembler>>,
    ) {
        let worker_tid: ThreadId = thread::current().id();

        loop {
            let message: Message = match channel_rx.recv() {
                Ok(VenvCommand::Work(message)) => message,
                Ok(VenvCommand::Shutdown) => {
                    debug!("handle_message(): thread received shutdown message (worker_tid={worker_tid:?})");
                    break
                },
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
                                    match Self::handle_special_messages(
                                        gateway_stdin_tx.clone(),
                                        gateway_stdout_tx.clone(),
                                        venv.clone(),
                                        source,
                                        message,
                                    ) {
                                        Ok(message) => message,
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            // WorkerThreadErrors other than Interrupted should be
                                            // caught by downstream functions, and converted to
                                            // Messages with the appropriate return code.
                                            unreachable!("fatal error in working thread (error={e:?})");
                                        }
                                    }
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
                                    match Self::handle_short_request_messages(source, message) {
                                        Ok(message) => message,
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            // WorkerThreadErrors other than Interrupted should be
                                            // caught by downstream functions, and converted to
                                            // Messages with the appropriate return code.
                                            unreachable!("fatal error in working thread (error={e:?})");
                                        }
                                    }
                                },

                                // The following system calls have their request data fit in a
                                // single message, but their response data is too large to fit in a
                                // single message. Thus, their response is split into multiple
                                // messages.
                                LinuxDaemonMessageHeader::FileStatRequest
                                | LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryRequest
                                | LinuxDaemonMessageHeader::GetDirectoryEntriesRequest => {
                                    match Self::handle_long_response_messages(
                                        uvm_stream.clone(),
                                        source,
                                        message,
                                    ) {
                                        Ok(message) => message,
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            // WorkerThreadErrors other than Interrupted should be
                                            // caught by downstream functions, and converted to
                                            // Messages with the appropriate return code.
                                            unreachable!("fatal error in working thread (error={e:?})");
                                        }
                                    }
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
                                    match Self::handle_long_request_messages(
                                        uvm_stream.clone(),
                                        assembler.clone(),
                                        source,
                                        message,
                                    ) {
                                        Ok(message) => message,
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            // WorkerThreadErrors other than Interrupted should be
                                            // caught by downstream functions, and converted to
                                            // Messages with the appropriate return code.
                                            unreachable!("fatal error in working thread (error={e:?})");
                                        }
                                    }
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
    ) -> Result<Message, WorkerThreadError> {
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
    ) -> Result<Message, WorkerThreadError> {
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
    ) -> Result<(), WorkerThreadError> {
        match message.header {
            LinuxDaemonMessageHeader::ChangeDirectoryRequestPart => {
                Self::handle_long_request::<ChangeDirectoryRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::FileAccessAtRequestPart => {
                Self::handle_long_request::<FileAccessAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::FileStatAtRequestPart => {
                Self::handle_long_request::<FileStatAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::SymbolicLinkAtRequestPart => {
                Self::handle_long_request::<SymbolicLinkAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::LinkAtRequestPart => {
                Self::handle_long_request::<LinkAtRequest>(uvm_stream, assembler, source, &message)
            },
            LinuxDaemonMessageHeader::ReadLinkAtRequestPart => {
                Self::handle_long_request::<ReadLinkAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::MakeDirectoryAtRequestPart => {
                Self::handle_long_request::<MakeDirectoryAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::UpdateFileAccessTimeAtRequestPart => {
                Self::handle_long_request::<UpdateFileAccessTimeAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::FileChownAtRequestPart => {
                Self::handle_long_request::<FileChownAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::FileChmodAtRequestPart => {
                Self::handle_long_request::<FileChmodAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::OpenAtRequestPart => {
                Self::handle_long_request::<OpenAtRequest>(uvm_stream, assembler, source, &message)
            },
            LinuxDaemonMessageHeader::RenameAtRequestPart => {
                Self::handle_long_request::<RenameAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
            },
            LinuxDaemonMessageHeader::UnlinkAtRequestPart => {
                Self::handle_long_request::<UnlinkAtRequest>(
                    uvm_stream, assembler, source, &message,
                )
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
    ) -> Result<(), WorkerThreadError> {
        match message.header {
            LinuxDaemonMessageHeader::FileStatRequest => {
                Self::handle_fstat_request(uvm_stream, source, message)
            },
            LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryRequest => {
                Self::handle_getcwd_request(uvm_stream, source)
            },
            LinuxDaemonMessageHeader::GetDirectoryEntriesRequest => {
                Self::handle_getdents_request(uvm_stream, source, message)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected long response message {:?}", header)
            },
        }
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

    fn handle_close_request(source: ThreadIdentifier, request: CloseRequest) -> Result<Message, WorkerThreadError> {
        // Inspect file descriptor that is being closed, as we need to
        // handle standard file descriptors specially.
        match request.fd {
            // Closing standard file descriptors.
            STDIN_FILENO | STDOUT_FILENO | STDERR_FILENO => {
                // Perform a fake close, as standard file descriptors
                // are shared with the current process.
                Ok(CloseResponse::build(source, 0))
            },
            // Closing other file descriptors.
            _ => unistd::do_close(source, request),
        }
    }

    fn handle_write_request(
        gateway_stdout_tx: Option<Sender<WriteRequest>>,
        source: ThreadIdentifier,
        mut request: WriteRequest,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_write_request(): source={source:?}, request={request:?}");
        // Check if writing to gateway.
        if request.fd == STDOUT_FILENO || request.fd == STDERR_FILENO {
            let gateway_stdout_tx = if let Some(gateway_stdout_tx) = gateway_stdout_tx {
                gateway_stdout_tx
            } else {
                error!("handle_write_request(): trying to write to stdout without a gateway configured");
                return Ok(build_error(source, ErrorCode::InvalidArgument));
            };

            // Check if write size is invalid.
            if request.count == 0 {
                // Writing zero-bytes to STDOUT is not allowed, as we used this to signal EOF.
                error!("handle_write_request(): trying to write zero bytes to STDOUT");
                Ok(build_error(source, ErrorCode::InvalidArgument))
            } else {
                profiler::timestamp_message!(&mut request.buffer, 0);
                let count: usize = request.count as usize;
                if let Err(error) = gateway_stdout_tx.send(request) {
                    debug!("failed to write buffer to the gateway (error={error:?})");
                    // TODO: Check error conversion.
                    return Ok(build_error(source, ErrorCode::ConnectionReset));
                }

                // We don't wait for the IO thread to confirm that the write was correct, as writes
                // are fully non-blocking.
                debug!("wrote {count} bytes to the gateway");
                Ok(WriteResponse::build(source, count as i32))
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
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_read_request(): source={source:?}, request={request:?}");
        // Check if reading from gateway.
        if request.fd == STDIN_FILENO {
            let gateway_stdin_tx = if let Some(gateway_stdin_tx) = gateway_stdin_tx {
                gateway_stdin_tx
            } else {
                error!("handle_read_request(): process tried to read from stdin but no gateway found");
                return Ok(ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE]));
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
                return Ok(ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE]));
            };

            // Send ReadRequest to gateway IO thread.
            if let Err(error) = gateway_stdin_tx.send((request, env.get_stdin_response_tx())) {
                error!(
                    "handle_read_request(): error sending request to gateway STDIN IO thread, returning EOF \
                    (error={error:?})"
                );
                return Ok(ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE]));
            }

            // Wait for response from IO thread.
            match env
                .get_stdin_response_rx()
                .recv() {
                    Ok(mut read_response) => {
                        // We don't have access to the source in the gateway IO thread, so we set
                        // it here.
                        read_response.destination = source.into();
                        Ok(read_response)
                    },
                    Err(e) => {
                        error!(
                            "handle_read_request(): error receiving request response from gateway STDIN \
                            IO thread, returning EOF (error={e:?})"
                        );
                        Ok(ReadResponse::build(source, 0, [0u8; ReadResponse::BUFFER_SIZE]))
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
    ) -> Result<(), WorkerThreadError> {
        let request: FileStatRequest = FileStatRequest::from_bytes(message.payload);
        let messages: Vec<Message> = fcntl::do_fstat(source, request)?;
        for message in messages {
            if let Err(e) = Self::send(uvm_stream.clone(), message) {
                error!("failed to send message (error={e:?})");
            }
        }

        Ok(())
    }

    fn handle_getcwd_request(uvm_stream: Arc<Mutex<SocketStream>>, source: ThreadIdentifier) -> Result<(), WorkerThreadError> {
        let messages: Vec<Message> = unistd::do_getcwd(source)?;
        for message in messages {
            if let Err(e) = Self::send(uvm_stream.clone(), message) {
                error!("failed to send message (error={e:?})");
            }
        }

        Ok(())
    }

    fn handle_getdents_request(
        uvm_stream: Arc<Mutex<SocketStream>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Result<(), WorkerThreadError> {
        let request: GetDirectoryEntriesRequest =
            GetDirectoryEntriesRequest::from_bytes(message.payload);

        let messages: Vec<Message> = dirent::do_getdents(source, request)?;
        for message in messages {
            if let Err(e) = Self::send(uvm_stream.clone(), message) {
                error!("failed to send message (error={e:?})");
            }
        }

        Ok(())
    }

    fn handle_long_request<T>(
        uvm_stream: Arc<Mutex<SocketStream>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        source: ThreadIdentifier,
        message: &LinuxDaemonMessage,
    ) -> Result<(), WorkerThreadError>
        where
        T: RequestAssemblerTrait,
    {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        trace!("handle_long_request(): source={source:?}, part={part:?}");

        let result: Result<Option<Vec<Message>>, WorkerThreadError> =
            assembler.lock().unwrap().process_message::<T>(source, part);

        match result {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) = Self::send(uvm_stream.clone(), message) {
                        error!("failed to send message (error={e:?})");
                    }
                }

                Ok(())
            },
            Ok(None) => Ok(()),
            Err(WorkerThreadError::Interrupted) => Err(WorkerThreadError::Interrupted),
            Err(WorkerThreadError::Error(e)) => {
                error!("failed to process request (error={e:?})");
                // TODO: proper error code conversion.
                if let Err(e) = Self::send(uvm_stream.clone(), Self::do_error(source, ErrorCode::IoErr)) {
                    error!("failed to send error message (error={e:?})");
                }

                Ok(())
            },
        }
    }
}
