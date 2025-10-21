// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    build_error,
    dirent,
    error::WorkerThreadError,
    fcntl,
    message::{
        RequestAssembler,
        RequestAssemblerTrait,
    },
    socket,
    sys_select,
    syscalls::SystemCallRouteTable,
    times,
    unistd,
    user_vm_handle::UserVmHandle,
    venv::VenvCommand,
};
use ::anyhow::Result;
use ::std::{
    io::ErrorKind,
    mem,
    ptr,
    sync::{
        atomic::{
            AtomicUsize,
            Ordering,
        },
        Arc,
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
    poll::message::PollRequest,
    sys::{
        select::message::SelectRequest,
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
use ::syscomm::{
    SocketStreamReader,
    SocketStreamWriter,
    WriteAll,
};
use ::syslog::{
    debug,
    error,
    trace,
};
use ::tokio::{
    runtime::Handle,
    sync::{
        mpsc::{
            Receiver,
            Sender,
        },
        Mutex,
        MutexGuard,
    },
    task::{
        self,
        JoinHandle,
    },
};
use libc::{
    c_int,
    pthread_kill,
    pthread_self,
    sigaction,
    sigemptyset,
    SIGUSR1,
};

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
    pub fn spawn_worker_thread(
        id: ThreadIdentifier,
        channel_rx: Receiver<VenvCommand>,
        channel_tx: Sender<VenvCommand>,
        uvm_handle: UserVmHandle,
        assembler: Arc<Mutex<RequestAssembler>>,
        syscall_table: Arc<SystemCallRouteTable>,
    ) -> Result<Self, Error> {
        trace!("spawning workker thread (id={id:?})");
        // We use an atomic to pass the id of the created thread back to the caller context. We
        // need this because std::thread's JoinHandle does not expose the tid.
        let pthread_id_holder = Arc::new(AtomicUsize::new(0));

        // Make copies to return as part of the thread handle.
        let pthread_id_holder_clone = pthread_id_holder.clone();

        let join_handle = task::spawn_blocking(move || {
            Self::install_signal_handler();

            let pthread_id = unsafe { pthread_self() };
            pthread_id_holder.store(pthread_id as usize, Ordering::Relaxed);

            Self::handle_message(channel_rx, uvm_handle, syscall_table, assembler);

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
        mut channel_rx: Receiver<VenvCommand>,
        uvm_handle: UserVmHandle,
        syscall_table: Arc<SystemCallRouteTable>,
        assembler: Arc<Mutex<RequestAssembler>>,
    ) {
        let worker_tid: ThreadId = thread::current().id();
        let uvm_stream: Arc<Mutex<SocketStreamWriter>> = uvm_handle.get_user_vm_writer();

        let (gateway_reader, gateway_writer) =
            match Handle::current().block_on(uvm_handle.get_gateway_vm_stream()) {
                Ok((reader, writer)) => (reader, writer),
                Err(e) => {
                    error!(
                        "handle_message(): failed to connect to gateway socket \
                         (worker_tid={worker_tid:?}, error={e:?})"
                    );
                    return;
                },
            };

        loop {
            let message: Message = match channel_rx.blocking_recv() {
                Some(VenvCommand::Work(message)) => message,
                Some(VenvCommand::Shutdown) => {
                    debug!(
                        "handle_message(): thread received shutdown message \
                         (worker_tid={worker_tid:?})"
                    );
                    break;
                },
                None => {
                    error!(
                        "handle_message(): failed to receive message from channel, stopping \
                         (worker_tid={worker_tid:?})"
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
                                        syscall_table.clone(),
                                        gateway_reader.clone(),
                                        gateway_writer.clone(),
                                        source,
                                        message,
                                    ) {
                                        Ok(message) => message,
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            // WorkerThreadErrors other than Interrupted should be
                                            // caught by downstream functions, and converted to
                                            // Messages with the appropriate return code.
                                            unreachable!(
                                                "fatal error in working thread (error={e:?})"
                                            );
                                        },
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
                                | LinuxDaemonMessageHeader::SelectRequest
                                | LinuxDaemonMessageHeader::SendSocketRequest
                                | LinuxDaemonMessageHeader::ShutdownSocketRequest
                                | LinuxDaemonMessageHeader::TimesRequest
                                | LinuxDaemonMessageHeader::PipeRequest
                                | LinuxDaemonMessageHeader::UpdateFileAccessTimeRequest => {
                                    match Self::handle_short_request_messages(
                                        syscall_table.clone(),
                                        source,
                                        message,
                                    ) {
                                        Ok(message) => message,
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            // WorkerThreadErrors other than Interrupted should be
                                            // caught by downstream functions, and converted to
                                            // Messages with the appropriate return code.
                                            unreachable!(
                                                "fatal error in working thread (error={e:?})"
                                            );
                                        },
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
                                        syscall_table.clone(),
                                        source,
                                        message,
                                    ) {
                                        Ok(message) => message,
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            // WorkerThreadErrors other than Interrupted should be
                                            // caught by downstream functions, and converted to
                                            // Messages with the appropriate return code.
                                            unreachable!(
                                                "fatal error in working thread (error={e:?})"
                                            );
                                        },
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
                                | LinuxDaemonMessageHeader::UnlinkAtRequestPart
                                | LinuxDaemonMessageHeader::PollRequestPart => {
                                    match Self::handle_long_request_messages(
                                        uvm_stream.clone(),
                                        assembler.clone(),
                                        syscall_table.clone(),
                                        source,
                                        message,
                                    ) {
                                        Ok(()) => {},
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            // WorkerThreadErrors other than Interrupted should be
                                            // caught by downstream functions, and converted to
                                            // Messages with the appropriate return code.
                                            unreachable!(
                                                "fatal error in working thread (error={e:?})"
                                            );
                                        },
                                    }
                                    continue;
                                },

                                _ => Self::do_error(source, ErrorCode::InvalidMessage),
                            };
                            match Handle::current()
                                .block_on(Self::send(uvm_stream.clone(), message))
                            {
                                Ok(()) => {},
                                Err(ref e) if e.kind() == ErrorKind::BrokenPipe => {
                                    debug!("user vm stream closed, worker thread exiting");
                                    break;
                                },
                                Err(e) => {
                                    // send only ever raises BrokenPipe errors.
                                    unreachable!(
                                        "handle_message: worker thread received unrecognized \
                                         error (error={e:?})"
                                    );
                                },
                            };
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
        syscall_table: Arc<SystemCallRouteTable>,
        gateway_reader: Arc<Mutex<SocketStreamReader>>,
        gateway_writer: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Result<Message, WorkerThreadError> {
        match message.header {
            LinuxDaemonMessageHeader::CloseRequest => {
                let request: CloseRequest = CloseRequest::from_bytes(message.payload);
                Self::handle_close_request(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ReadRequest => {
                let request: ReadRequest = ReadRequest::from_bytes(message.payload);
                Self::handle_read_request(syscall_table, gateway_reader, source, request)
            },
            LinuxDaemonMessageHeader::WriteRequest => {
                let request: WriteRequest = WriteRequest::from_bytes(message.payload);
                Self::handle_write_request(syscall_table, gateway_writer, source, request)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected special message {:?}", header)
            },
        }
    }

    fn handle_short_request_messages(
        syscall_table: Arc<SystemCallRouteTable>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Result<Message, WorkerThreadError> {
        match message.header {
            LinuxDaemonMessageHeader::AcceptSocketRequest => {
                let request: AcceptSocketRequest = AcceptSocketRequest::from_bytes(message.payload);
                socket::do_accept(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::BindSocketRequest => {
                let request: BindSocketRequest = BindSocketRequest::from_bytes(message.payload);
                socket::do_bind(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ConnectSocketRequest => {
                let request: ConnectSocketRequest =
                    ConnectSocketRequest::from_bytes(message.payload);
                socket::do_connect(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::CreateSocketPairRequest => {
                let request: CreateSocketPairRequest =
                    CreateSocketPairRequest::from_bytes(message.payload);
                socket::do_socketpair(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::CreateSocketRequest => {
                let request: CreateSocketRequest = CreateSocketRequest::from_bytes(message.payload);
                socket::do_socket(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileAdvisoryInformationRequest => {
                let request: FileAdvisoryInformationRequest =
                    FileAdvisoryInformationRequest::from_bytes(message.payload);
                fcntl::do_posix_fadvise(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileChdirRequest => {
                let request: FileChdirRequest = FileChdirRequest::from_bytes(message.payload);
                unistd::do_fchdir(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileChmodRequest => {
                let request: FileChmodRequest = FileChmodRequest::from_bytes(message.payload);
                fcntl::do_fchmod(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileChownRequest => {
                let request: FileChownRequest = FileChownRequest::from_bytes(message.payload);
                unistd::do_fchown(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileControlRequest => {
                let request: FileControlRequest = FileControlRequest::from_bytes(message.payload);
                fcntl::do_fcntl(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileDataSyncRequest => {
                let request: FileDataSyncRequest = FileDataSyncRequest::from_bytes(message.payload);
                unistd::do_fdatasync(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileSpaceControlRequest => {
                let request: FileSpaceControlRequest =
                    FileSpaceControlRequest::from_bytes(message.payload);
                fcntl::do_posix_fallocate(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileSyncRequest => {
                let request: FileSyncRequest = FileSyncRequest::from_bytes(message.payload);
                unistd::do_fsync(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileTruncateRequest => {
                let request: FileTruncateRequest = FileTruncateRequest::from_bytes(message.payload);
                unistd::do_ftruncate(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::GetIdsRequest => {
                let request: GetIdsRequest = GetIdsRequest::from_bytes(message.payload);
                unistd::do_getids(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::GetPeerNameRequest => {
                let request: GetPeerNameRequest = GetPeerNameRequest::from_bytes(message.payload);
                socket::do_getpeername(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::GetSockNameRequest => {
                let request: GetSockNameRequest = GetSockNameRequest::from_bytes(message.payload);
                socket::do_getsockname(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ListenSocketRequest => {
                let request: ListenSocketRequest = ListenSocketRequest::from_bytes(message.payload);
                socket::do_listen(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::PartialReadRequest => {
                let request: PartialReadRequest = PartialReadRequest::from_bytes(message.payload);
                unistd::do_pread(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::PartialWriteRequest => {
                let request: PartialWriteRequest = PartialWriteRequest::from_bytes(message.payload);
                unistd::do_pwrite(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ReceiveSocketRequest => {
                let request: ReceiveSocketRequest =
                    ReceiveSocketRequest::from_bytes(message.payload);
                socket::do_recv(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::SeekRequest => {
                let request: SeekRequest = SeekRequest::from_bytes(message.payload);
                unistd::do_lseek(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::SelectRequest => {
                let request: SelectRequest = SelectRequest::from_bytes(message.payload);
                sys_select::do_select(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::SendSocketRequest => {
                let request: SendSocketRequest = SendSocketRequest::from_bytes(message.payload);
                socket::do_send(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ShutdownSocketRequest => {
                let request: ShutdownSocketRequest =
                    ShutdownSocketRequest::from_bytes(message.payload);
                socket::do_shutdown(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::TimesRequest => {
                let request: TimesRequest = TimesRequest::from_bytes(message.payload);
                times::do_times(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::UpdateFileAccessTimeRequest => {
                let request: UpdateFileAccessTimeRequest =
                    UpdateFileAccessTimeRequest::from_bytes(message.payload);
                fcntl::do_futimens(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::PipeRequest => {
                let _request = PipeRequest::from_bytes(message.payload);
                unistd::do_pipe(syscall_table, source)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected short message {:?}", header)
            },
        }
    }

    fn handle_long_request_messages(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        syscall_table: Arc<SystemCallRouteTable>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Result<(), WorkerThreadError> {
        match message.header {
            LinuxDaemonMessageHeader::ChangeDirectoryRequestPart => {
                Self::handle_long_request::<ChangeDirectoryRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::FileAccessAtRequestPart => {
                Self::handle_long_request::<FileAccessAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::FileStatAtRequestPart => {
                Self::handle_long_request::<FileStatAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::SymbolicLinkAtRequestPart => {
                Self::handle_long_request::<SymbolicLinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::LinkAtRequestPart => {
                Self::handle_long_request::<LinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::ReadLinkAtRequestPart => {
                Self::handle_long_request::<ReadLinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::MakeDirectoryAtRequestPart => {
                Self::handle_long_request::<MakeDirectoryAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::UpdateFileAccessTimeAtRequestPart => {
                Self::handle_long_request::<UpdateFileAccessTimeAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::FileChownAtRequestPart => {
                Self::handle_long_request::<FileChownAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::FileChmodAtRequestPart => {
                Self::handle_long_request::<FileChmodAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::OpenAtRequestPart => {
                Self::handle_long_request::<OpenAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::RenameAtRequestPart => {
                Self::handle_long_request::<RenameAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::UnlinkAtRequestPart => {
                Self::handle_long_request::<UnlinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::PollRequestPart => Self::handle_long_request::<PollRequest>(
                uvm_stream,
                assembler,
                syscall_table,
                source,
                &message,
            ),
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected long request message {:?}", header)
            },
        }
    }

    fn handle_long_response_messages(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        syscall_table: Arc<SystemCallRouteTable>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Result<(), WorkerThreadError> {
        match message.header {
            LinuxDaemonMessageHeader::FileStatRequest => {
                Self::handle_fstat_request(syscall_table, uvm_stream, source, message)
            },
            LinuxDaemonMessageHeader::GetCurrentWorkingDirectoryRequest => {
                Self::handle_getcwd_request(uvm_stream, syscall_table, source)
            },
            LinuxDaemonMessageHeader::GetDirectoryEntriesRequest => {
                Self::handle_getdents_request(syscall_table, uvm_stream, source, message)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected long response message {:?}", header)
            },
        }
    }

    async fn send(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        message: Message,
    ) -> Result<(), std::io::Error> {
        let mut guard: MutexGuard<'_, SocketStreamWriter> = uvm_stream.lock().await;
        guard.write_all(&message.to_bytes()).await?;
        Ok(())
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

    fn handle_close_request(
        syscall_table: Arc<SystemCallRouteTable>,
        source: ThreadIdentifier,
        request: CloseRequest,
    ) -> Result<Message, WorkerThreadError> {
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
            _ => unistd::do_close(syscall_table, source, request),
        }
    }

    fn handle_write_request(
        syscall_table: Arc<SystemCallRouteTable>,
        gateway_writer: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        mut request: WriteRequest,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_write_request(): source={source:?}, request={request:?}");
        // Check if writing to gateway.
        if request.fd == STDOUT_FILENO || request.fd == STDERR_FILENO {
            // Check if write size is invalid.
            if request.count == 0 {
                // Writing zero-bytes to STDOUT is not allowed, as we used this to signal EOF.
                error!("handle_write_request(): trying to write zero bytes to STDOUT");
                Ok(build_error(source, ErrorCode::InvalidArgument))
            } else {
                // Label: linuxd::worker_thread::handle_write_request()
                profiler::timestamp_message!(&mut request.buffer, 0);
                let count: usize = request.count as usize;

                let mut locked_gateway_writer = gateway_writer.blocking_lock();
                let mut total_written: usize = 0;

                while total_written < count {
                    let write_slice: &[u8] = &request.buffer[total_written..count];
                    match Handle::current().block_on(locked_gateway_writer.write(write_slice)) {
                        Ok(0) => {
                            error!("failed to write to gateway socket (written={total_written})");
                            return Ok(build_error(source, ErrorCode::IoErr));
                        },
                        Ok(n) => {
                            total_written += n;
                        },
                        Err(e) if e.kind() == ErrorKind::Interrupted => {
                            return Err(WorkerThreadError::Interrupted);
                        },
                        Err(e) => {
                            error!("failed to write to gateway socket (error={e:?})");
                            return Ok(build_error(source, ErrorCode::IoErr));
                        },
                    }
                }

                // We don't wait for the IO thread to confirm that the write was correct, as writes
                // are fully non-blocking.
                debug!("wrote {count} bytes to the gateway");
                Ok(WriteResponse::build(source, count as i32))
            }
        } else {
            // Write to other file descriptor.
            unistd::do_write(syscall_table, source, request)
        }
    }

    fn handle_read_request(
        syscall_table: Arc<SystemCallRouteTable>,
        gateway_reader: Arc<Mutex<SocketStreamReader>>,
        source: ThreadIdentifier,
        request: ReadRequest,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_read_request(): source={source:?}, request={request:?}");
        // Check if reading from gateway.
        if request.fd == STDIN_FILENO {
            // We need a mutable message to be able to timestamp it during profiling of the data
            // path. The profiler macro is designed to silence this warnings when compiled without
            // the timestamp-messages macro. However, in this case we need to unwrap the message
            // before timestamping, wrapping the macro itself in a #[cfg()]. This means we need to
            // manually silence the warning.
            #[allow(unused_mut)]
            let mut response: Result<Message, WorkerThreadError> = {
                // Take the lock.
                let mut locked_gateway_reader: MutexGuard<'_, SocketStreamReader> =
                    gateway_reader.blocking_lock();

                // Read from the gateway.
                let mut response_buf: [u8; ReadResponse::BUFFER_SIZE] =
                    [0u8; ReadResponse::BUFFER_SIZE];
                // Blocking read from the gateway socket to make sure we can be interrupted if
                // necessary.
                match Handle::current().block_on(locked_gateway_reader.read(&mut response_buf)) {
                    Ok(0) => {
                        debug!("handle_read_request(): eof");
                        Ok(ReadResponse::eof(source))
                    },
                    Ok(n) => {
                        debug!("read {n} bytes from gateway: {response_buf:?}");
                        Ok(ReadResponse::build(source, n as c_ssize_t, response_buf))
                    },
                    Err(e) if e.kind() == ErrorKind::Interrupted => {
                        return Err(WorkerThreadError::Interrupted)
                    },
                    Err(e) => {
                        error!(
                            "handle_read_request(): error reading data from gateway (error={e:?})"
                        );
                        Ok(ReadResponse::eof(source))
                    },
                }
            };

            #[cfg(feature = "timestamp-messages")]
            if let Ok(read_response) = &mut response {
                // Label: linuxd::worker_thread::handle_read_request()
                profiler::timestamp_message!(
                    &mut read_response.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                );
            }

            response
        } else {
            // Read from other file descriptor.
            unistd::do_read(syscall_table, source, request)
        }
    }

    fn handle_fstat_request(
        syscall_table: Arc<SystemCallRouteTable>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Result<(), WorkerThreadError> {
        let request: FileStatRequest = FileStatRequest::from_bytes(message.payload);
        let messages: Vec<Message> = fcntl::do_fstat(syscall_table, source, request)?;
        trace!("handle_fstat_request(): obtained {} messages", messages.len());
        for message in messages {
            trace!("handle_fstat_request(): sending message: {message:?}");
            if let Err(e) = Handle::current().block_on(Self::send(uvm_stream.clone(), message)) {
                error!("failed to send message (error={e:?})");
            }
        }

        Ok(())
    }

    fn handle_getcwd_request(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        syscall_table: Arc<SystemCallRouteTable>,
        source: ThreadIdentifier,
    ) -> Result<(), WorkerThreadError> {
        let messages: Vec<Message> = unistd::do_getcwd(syscall_table, source)?;
        for message in messages {
            if let Err(e) = Handle::current().block_on(Self::send(uvm_stream.clone(), message)) {
                error!("failed to send message (error={e:?})");
            }
        }

        Ok(())
    }

    fn handle_getdents_request(
        syscall_table: Arc<SystemCallRouteTable>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Result<(), WorkerThreadError> {
        let request: GetDirectoryEntriesRequest =
            GetDirectoryEntriesRequest::from_bytes(message.payload);

        let messages: Vec<Message> = dirent::do_getdents(syscall_table, source, request)?;
        for message in messages {
            if let Err(e) = Handle::current().block_on(Self::send(uvm_stream.clone(), message)) {
                error!("failed to send message (error={e:?})");
            }
        }

        Ok(())
    }

    fn handle_long_request<T>(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        syscall_table: Arc<SystemCallRouteTable>,
        source: ThreadIdentifier,
        message: &LinuxDaemonMessage,
    ) -> Result<(), WorkerThreadError>
    where
        T: RequestAssemblerTrait,
    {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        trace!("handle_long_request(): source={source:?}, part={part:?}");

        let result: Result<Option<Vec<Message>>, WorkerThreadError> = assembler
            .blocking_lock()
            .process_message::<T>(syscall_table, source, part);

        match result {
            Ok(Some(messages)) => {
                for message in messages {
                    if let Err(e) =
                        Handle::current().block_on(Self::send(uvm_stream.clone(), message))
                    {
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
                if let Err(e) = Handle::current().block_on(Self::send(
                    uvm_stream.clone(),
                    Self::do_error(source, ErrorCode::IoErr),
                )) {
                    error!("failed to send error message (error={e:?})");
                }

                Ok(())
            },
        }
    }
}
