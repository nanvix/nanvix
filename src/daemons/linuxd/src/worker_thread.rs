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
    error::WorkerThreadError,
    linux::{
        dirent,
        fcntl,
        sys_select,
        sys_socket,
        sys_times,
        unistd,
    },
    message::{
        RequestAssembler,
        RequestAssemblerTrait,
    },
    syscalls::SyscallTable,
    shared_ring::SharedRing,
    user_vm_handle::UserVmHandle,
    venv::VenvCommand,
};
use ::anyhow::Result;
use ::libc::{
    c_int,
    pthread_kill,
    pthread_self,
    sigaction,
    sigemptyset,
    SIGUSR1,
};
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::std::{
    io::ErrorKind,
    mem,
    ptr,
    sync::{
        atomic::{
            AtomicBool,
            AtomicUsize,
            Ordering,
        },
        Arc,
    },
    thread::{
        self,
        ThreadId,
    },
    time::Duration,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        FixedBufferTransfer,
        IkcFrame,
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
        PositionedReadRequest,
        PositionedWriteRequest,
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

//==================================================================================================
// Constants
//==================================================================================================

/// Signal used to interrupt async operations in worker threads.
const INTERRUPT_SIGNAL: c_int = SIGUSR1;

/// Frequency to poll for cancellation flag.
const CANCELLATION_POLL_FREQUENCY: Duration = Duration::from_millis(100);

/// Maximum time (in seconds) a worker thread will wait for the bulk data message that must follow
/// a bulk-transfer request (`ReadRequest`, `WriteRequest`, `PositionedReadRequest`, or
/// `PositionedWriteRequest`). If the guest VM crashes (or the channel stalls) after sending the IKC
/// request but before the corresponding push/pull arrives, this timeout prevents the worker thread
/// from blocking forever.
const BULK_DATA_TIMEOUT: Duration = Duration::from_secs(30);

//==================================================================================================
// Thread-Local Storage
//==================================================================================================

thread_local! {
    /// Atomic flag for interrupting async operations in the current worker thread.
    /// This flag is set by the signal handler to indicate that the thread should be cancelled.
    static CANCELLATION_FLAG: AtomicBool = const { AtomicBool::new(false) };
}

//==================================================================================================
// Structures
//==================================================================================================

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

//==================================================================================================
// Implementations
//==================================================================================================

impl WorkerThreadHandle {
    fn install_signal_handler() {
        // SAFETY: we install a signal handler that is a no-op so this is safe.
        let ret = unsafe {
            let sig_action = sigaction {
                sa_sigaction: linuxd_worker_thread_signal_handler as *const () as usize,
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

    ///
    /// # Description
    ///
    /// Run an async operation with cancellation support via the thread-local cancellation flag.
    ///
    /// This helper function wraps an async operation to make it interruptible via the
    /// thread-local cancellation flag. When the flag is set (typically by a signal handler),
    /// the operation is cancelled and returns `ErrorKind::Interrupted`.
    ///
    /// # Parameters
    ///
    /// * `f` - The async operation to run. Must return a `std::io::Result<R>`.
    ///
    /// # Returns
    ///
    /// Returns the result of the async operation, or an `Interrupted` error if cancelled.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The operation is cancelled via the cancellation flag
    /// - The underlying operation fails
    ///
    /// # Safety
    ///
    /// Note: The operation `f` may not be cancel-safe. This is acceptable in our use case because
    /// cancellation only occurs when a signal interrupts the operation, ensuring that partial
    /// completion is handled by the underlying I/O layer. The signal is sent externally to
    /// interrupt blocking system calls, making cancellation safe.
    ///
    fn run_cancellable_operation<F, R>(f: F) -> ::std::io::Result<R>
    where
        F: ::std::future::Future<Output = ::std::io::Result<R>>,
    {
        Handle::current().block_on(async {
            ::tokio::pin!(f);

            // Poll the cancellation flag periodically while waiting for the future.
            loop {
                ::tokio::select! {
                    result = &mut f => return result,
                    _ = ::tokio::time::sleep(CANCELLATION_POLL_FREQUENCY) => {
                        // Check if cancellation was requested.
                        let cancelled: bool = CANCELLATION_FLAG.with(|flag| flag.load(Ordering::Relaxed));
                        if cancelled {
                            return Err(::std::io::Error::new(
                                ErrorKind::Interrupted,
                                "run_cancellable_operation(): operation cancelled by signal",
                            ));
                        }
                    }
                }
            }
        })
    }

    ///
    /// # Description
    ///
    /// Receives the next [`VenvCommand`] from the worker channel with a timeout and cancellation
    /// support. This prevents the worker thread from blocking indefinitely when the guest VM
    /// crashes after sending an IKC request but before sending the corresponding bulk data.
    ///
    /// # Parameters
    ///
    /// - `channel_rx`: The channel receiver to read from.
    /// - `timeout`: Maximum duration to wait for the message.
    ///
    /// # Returns
    ///
    /// The received command, or a [`WorkerThreadError`] if the operation times out, the channel
    /// closes, or the thread is cancelled.
    ///
    fn recv_with_timeout(
        channel_rx: &mut Receiver<VenvCommand>,
        timeout: Duration,
    ) -> Result<VenvCommand, WorkerThreadError> {
        Handle::current().block_on(async {
            match ::tokio::time::timeout(timeout, channel_rx.recv()).await {
                Ok(Some(cmd)) => Ok(cmd),
                Ok(None) => {
                    error!("recv_with_timeout(): channel closed");
                    Err(WorkerThreadError::Interrupted)
                },
                Err(_elapsed) => {
                    error!(
                        "recv_with_timeout(): timed out after {timeout:?} waiting for bulk data"
                    );
                    Err(WorkerThreadError::Interrupted)
                },
            }
        })
    }

    /// Spawn an interruptible worker thread.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_worker_thread<T: Sync + Send + 'static>(
        id: ThreadIdentifier,
        channel_rx: Receiver<VenvCommand>,
        channel_tx: Sender<VenvCommand>,
        uvm_handle: UserVmHandle,
        assembler: Arc<Mutex<RequestAssembler>>,
        syscall_table: Arc<SyscallTable<T>>,
    ) -> Result<Self, Error> {
        trace!("spawning worker thread (id={id:?})");
        // We use an atomic to pass the id of the created thread back to the caller context. We
        // need this because std::thread's JoinHandle does not expose the tid.
        let pthread_id_holder = Arc::new(AtomicUsize::new(0));

        // Make copies to return as part of the thread handle.
        let pthread_id_holder_clone = pthread_id_holder.clone();

        let join_handle = task::spawn_blocking(move || {
            let pthread_id = unsafe { pthread_self() };
            pthread_id_holder.store(pthread_id as usize, Ordering::Relaxed);

            // Initialize the cancellation flag for this worker thread before installing signal
            // handler to avoid race conditions.
            CANCELLATION_FLAG.with(|flag| {
                flag.store(false, Ordering::Relaxed);
            });

            Self::install_signal_handler();

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

        // SAFETY: We call pthread_kill() on a valid pthread_t that was stored when the thread was
        // created via spawn_blocking(). The thread ID is non-zero and the thread is guaranteed to
        // be alive because we hold a reference to its JoinHandle. The signal SIGUSR1 has a
        // handler installed that safely triggers cancellation via the thread-local token.
        let pthread_id = raw_tid as libc::pthread_t;
        unsafe { pthread_kill(pthread_id, INTERRUPT_SIGNAL) };

        Ok(())
    }

    fn handle_message<T>(
        mut channel_rx: Receiver<VenvCommand>,
        uvm_handle: UserVmHandle,
        syscall_table: Arc<SyscallTable<T>>,
        assembler: Arc<Mutex<RequestAssembler>>,
    ) {
        let worker_tid: ThreadId = thread::current().id();
        let uvm_stream: Arc<Mutex<SocketStreamWriter>> = uvm_handle.get_user_vm_writer();
        let shared_ring: Option<Arc<SharedRing>> = uvm_handle.shared_ring();

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
                Some(VenvCommand::BulkData(_)) => {
                    // Bulk data without a preceding Work message is unexpected; skip it.
                    warn!(
                        "handle_message(): received unexpected bulk data without request \
                         (worker_tid={worker_tid:?})"
                    );
                    continue;
                },
                Some(VenvCommand::FixedBuffer(_)) => {
                    warn!(
                        "handle_message(): received unexpected fixed-buffer transfer without request \
                         (worker_tid={worker_tid:?})"
                    );
                    continue;
                },
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
                sys::ipc::MessageType::Interrupt => {
                    error!("handle_message(): received unexpected interrupt message, stopping");
                    break;
                },
                sys::ipc::MessageType::Exception => {
                    error!("handle_message(): received unexpected exception message, stopping");
                    break;
                },
                sys::ipc::MessageType::Ipc => {
                    error!("handle_message(): received unexpected IPC message, stopping");
                    break;
                },
                sys::ipc::MessageType::ProcessTerminationEvent => {
                    error!(
                        "handle_message(): received unexpected process termination event, stopping"
                    );
                    break;
                },
                sys::ipc::MessageType::PullResponse => {
                    error!("handle_message(): received unexpected pull response, stopping");
                    break;
                },
                sys::ipc::MessageType::Ikc => {
                    match LinuxDaemonMessage::try_from_bytes(message.payload) {
                        Ok(message) => {
                            let message: Message = match message.header {
                                // The system calls are interposed before being forwarded to the
                                // backend provider.
                                LinuxDaemonMessageHeader::CloseRequest
                                | LinuxDaemonMessageHeader::ReadRequest
                                | LinuxDaemonMessageHeader::WriteRequest
                                | LinuxDaemonMessageHeader::PositionedReadRequest
                                | LinuxDaemonMessageHeader::PositionedWriteRequest => {
                                    match Self::handle_special_messages(
                                        &syscall_table,
                                        gateway_reader.clone(),
                                        gateway_writer.clone(),
                                        source,
                                        message,
                                        &mut channel_rx,
                                        uvm_stream.clone(),
                                        shared_ring.clone(),
                                    ) {
                                        Ok(message) => message,
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            error!(
                                                "handle_message(): fatal error in worker thread, \
                                                 stopping (error={e:?})"
                                            );
                                            break;
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
                                            error!(
                                                "handle_message(): fatal error in worker thread, \
                                                 stopping (error={e:?})"
                                            );
                                            break;
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
                                        &syscall_table,
                                        source,
                                        message,
                                    ) {
                                        Ok(message) => message,
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            error!(
                                                "handle_message(): fatal error in worker thread, \
                                                 stopping (error={e:?})"
                                            );
                                            break;
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
                                        &syscall_table,
                                        source,
                                        message,
                                    ) {
                                        Ok(()) => {},
                                        Err(WorkerThreadError::Interrupted) => break,
                                        Err(WorkerThreadError::Error(e)) => {
                                            error!(
                                                "handle_message(): fatal error in worker thread, \
                                                 stopping (error={e:?})"
                                            );
                                            break;
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

    fn handle_special_messages<T>(
        syscall_table: &SyscallTable<T>,
        gateway_reader: Arc<Mutex<SocketStreamReader>>,
        gateway_writer: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
        channel_rx: &mut Receiver<VenvCommand>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        shared_ring: Option<Arc<SharedRing>>,
    ) -> Result<Message, WorkerThreadError> {
        match message.header {
            LinuxDaemonMessageHeader::CloseRequest => {
                let request: CloseRequest = CloseRequest::from_bytes(message.payload);
                Self::handle_close_request(syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ReadRequest => {
                let request: ReadRequest = ReadRequest::from_bytes(message.payload);
                Self::handle_read_request(
                    syscall_table,
                    gateway_reader,
                    source,
                    request,
                    channel_rx,
                    uvm_stream,
                    shared_ring,
                )
            },
            LinuxDaemonMessageHeader::WriteRequest => {
                let request: WriteRequest = WriteRequest::from_bytes(message.payload);
                Self::handle_write_request(
                    syscall_table,
                    gateway_writer,
                    source,
                    request,
                    channel_rx,
                    shared_ring,
                )
            },
            LinuxDaemonMessageHeader::PositionedReadRequest => {
                let request: PositionedReadRequest = PositionedReadRequest::from_bytes(message.payload);
                Self::handle_positioned_read_request(
                    syscall_table,
                    source,
                    request,
                    channel_rx,
                    uvm_stream,
                    shared_ring,
                )
            },
            LinuxDaemonMessageHeader::PositionedWriteRequest => {
                let request: PositionedWriteRequest =
                    PositionedWriteRequest::from_bytes(message.payload);
                Self::handle_positioned_write_request(
                    syscall_table,
                    source,
                    request,
                    channel_rx,
                    shared_ring,
                )
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected special message {:?}", header)
            },
        }
    }

    fn handle_short_request_messages<T>(
        syscall_table: Arc<SyscallTable<T>>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Result<Message, WorkerThreadError> {
        match message.header {
            LinuxDaemonMessageHeader::AcceptSocketRequest => {
                let request: AcceptSocketRequest = AcceptSocketRequest::from_bytes(message.payload);
                sys_socket::do_accept(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::BindSocketRequest => {
                let request: BindSocketRequest = BindSocketRequest::from_bytes(message.payload);
                sys_socket::do_bind(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ConnectSocketRequest => {
                let request: ConnectSocketRequest =
                    ConnectSocketRequest::from_bytes(message.payload);
                sys_socket::do_connect(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::CreateSocketPairRequest => {
                let request: CreateSocketPairRequest =
                    CreateSocketPairRequest::from_bytes(message.payload);
                sys_socket::do_socketpair(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::CreateSocketRequest => {
                let request: CreateSocketRequest = CreateSocketRequest::from_bytes(message.payload);
                sys_socket::do_socket(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileAdvisoryInformationRequest => {
                let request: FileAdvisoryInformationRequest =
                    FileAdvisoryInformationRequest::from_bytes(message.payload);
                fcntl::do_posix_fadvise(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileChdirRequest => {
                let request: FileChdirRequest = FileChdirRequest::from_bytes(message.payload);
                unistd::do_fchdir(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileChmodRequest => {
                let request: FileChmodRequest = FileChmodRequest::from_bytes(message.payload);
                fcntl::do_fchmod(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileChownRequest => {
                let request: FileChownRequest = FileChownRequest::from_bytes(message.payload);
                unistd::do_fchown(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileControlRequest => {
                let request: FileControlRequest = FileControlRequest::from_bytes(message.payload);
                fcntl::do_fcntl(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileDataSyncRequest => {
                let request: FileDataSyncRequest = FileDataSyncRequest::from_bytes(message.payload);
                unistd::do_fdatasync(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileSpaceControlRequest => {
                let request: FileSpaceControlRequest =
                    FileSpaceControlRequest::from_bytes(message.payload);
                fcntl::do_posix_fallocate(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileSyncRequest => {
                let request: FileSyncRequest = FileSyncRequest::from_bytes(message.payload);
                unistd::do_fsync(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::FileTruncateRequest => {
                let request: FileTruncateRequest = FileTruncateRequest::from_bytes(message.payload);
                unistd::do_ftruncate(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::GetIdsRequest => {
                let request: GetIdsRequest = GetIdsRequest::from_bytes(message.payload);
                unistd::do_getids(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::GetPeerNameRequest => {
                let request: GetPeerNameRequest = GetPeerNameRequest::from_bytes(message.payload);
                sys_socket::do_getpeername(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::GetSockNameRequest => {
                let request: GetSockNameRequest = GetSockNameRequest::from_bytes(message.payload);
                sys_socket::do_getsockname(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ListenSocketRequest => {
                let request: ListenSocketRequest = ListenSocketRequest::from_bytes(message.payload);
                sys_socket::do_listen(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::PartialReadRequest => {
                let request: PartialReadRequest = PartialReadRequest::from_bytes(message.payload);
                unistd::do_pread(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::PartialWriteRequest => {
                let request: PartialWriteRequest = PartialWriteRequest::from_bytes(message.payload);
                unistd::do_pwrite(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ReceiveSocketRequest => {
                let request: ReceiveSocketRequest =
                    ReceiveSocketRequest::from_bytes(message.payload);
                sys_socket::do_recv(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::SeekRequest => {
                let request: SeekRequest = SeekRequest::from_bytes(message.payload);
                unistd::do_lseek(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::SelectRequest => {
                let request: SelectRequest = SelectRequest::from_bytes(message.payload);
                sys_select::do_select(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::SendSocketRequest => {
                let request: SendSocketRequest = SendSocketRequest::from_bytes(message.payload);
                sys_socket::do_send(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::ShutdownSocketRequest => {
                let request: ShutdownSocketRequest =
                    ShutdownSocketRequest::from_bytes(message.payload);
                sys_socket::do_shutdown(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::TimesRequest => {
                let request: TimesRequest = TimesRequest::from_bytes(message.payload);
                sys_times::do_times(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::UpdateFileAccessTimeRequest => {
                let request: UpdateFileAccessTimeRequest =
                    UpdateFileAccessTimeRequest::from_bytes(message.payload);
                fcntl::do_futimens(&syscall_table, source, request)
            },
            LinuxDaemonMessageHeader::PipeRequest => {
                let _request = PipeRequest::from_bytes(message.payload);
                unistd::do_pipe(&syscall_table, source)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected short message {:?}", header)
            },
        }
    }

    fn handle_long_request_messages<T>(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        message: LinuxDaemonMessage,
    ) -> Result<(), WorkerThreadError> {
        match message.header {
            LinuxDaemonMessageHeader::ChangeDirectoryRequestPart => {
                Self::handle_long_request::<T, ChangeDirectoryRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::FileAccessAtRequestPart => {
                Self::handle_long_request::<T, FileAccessAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::FileStatAtRequestPart => {
                Self::handle_long_request::<T, FileStatAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::SymbolicLinkAtRequestPart => {
                Self::handle_long_request::<T, SymbolicLinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::LinkAtRequestPart => {
                Self::handle_long_request::<T, LinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::ReadLinkAtRequestPart => {
                Self::handle_long_request::<T, ReadLinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::MakeDirectoryAtRequestPart => {
                Self::handle_long_request::<T, MakeDirectoryAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::UpdateFileAccessTimeAtRequestPart => {
                Self::handle_long_request::<T, UpdateFileAccessTimeAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::FileChownAtRequestPart => {
                Self::handle_long_request::<T, FileChownAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::FileChmodAtRequestPart => {
                Self::handle_long_request::<T, FileChmodAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::OpenAtRequestPart => {
                Self::handle_long_request::<T, OpenAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::RenameAtRequestPart => {
                Self::handle_long_request::<T, RenameAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::UnlinkAtRequestPart => {
                Self::handle_long_request::<T, UnlinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            LinuxDaemonMessageHeader::PollRequestPart => {
                Self::handle_long_request::<T, PollRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected long request message {:?}", header)
            },
        }
    }

    fn handle_long_response_messages<T>(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        syscall_table: &SyscallTable<T>,
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
        guard.write_all(&[IkcFrame::MESSAGE_FRAME]).await?;
        guard.write_all(&message.to_bytes()).await?;
        Ok(())
    }

    /// Sends a data chunk transfer to the user VM. The frame is: frame type byte + 4-byte LE length
    /// prefix + serialized DataChunk payload (header + data).
    async fn send_bulk(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        bulk: &::sys::ipc::DataChunk,
    ) -> Result<(), std::io::Error> {
        let payload: Vec<u8> = bulk.to_bytes();
        let payload_len: u32 = u32::try_from(payload.len()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "bulk payload length exceeds u32")
        })?;
        let len_prefix: [u8; 4] = payload_len.to_le_bytes();
        let mut guard: MutexGuard<'_, SocketStreamWriter> = uvm_stream.lock().await;
        guard.write_all(&[IkcFrame::DATA_CHUNK_FRAME]).await?;
        guard.write_all(&len_prefix).await?;
        guard.write_all(&payload).await?;
        Ok(())
    }

    async fn send_fixed(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        transfer: &FixedBufferTransfer,
    ) -> Result<(), std::io::Error> {
        let mut guard: MutexGuard<'_, SocketStreamWriter> = uvm_stream.lock().await;
        guard.write_all(&[IkcFrame::FIXED_BUFFER_FRAME]).await?;
        guard.write_all(&transfer.to_bytes()).await?;
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

    fn handle_close_request<T>(
        syscall_table: &SyscallTable<T>,
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

    fn handle_write_request<T>(
        syscall_table: &SyscallTable<T>,
        gateway_writer: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        request: WriteRequest,
        channel_rx: &mut Receiver<VenvCommand>,
        shared_ring: Option<Arc<SharedRing>>,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_write_request(): source={source:?}, request={request:?}");

        // Receive bulk data that carries the actual write payload. A timeout prevents the worker
        // thread from blocking forever if the guest VM crashes mid-protocol.
        let mut bulk_data: Option<Vec<u8>> = None;
        let mut fixed_buffer_ptr: Option<*mut u8> = None;
        let fixed_buffer_len: usize = match Self::recv_with_timeout(channel_rx, BULK_DATA_TIMEOUT) {
            Ok(VenvCommand::BulkData(bulk)) => {
                let mut data: Vec<u8> = bulk.into_data();
                profiler::timestamp_message!(&mut data, 0);
                let len: usize = data.len();
                bulk_data = Some(data);
                len
            },
            Ok(VenvCommand::FixedBuffer(fixed)) => {
                let Some(shared_ring) = shared_ring else {
                    error!(
                        "handle_write_request(): received fixed-buffer transfer without shared ring mapping"
                    );
                    return Ok(build_error(source, ErrorCode::InvalidMessage));
                };
                let ptr: *mut u8 = match shared_ring.fixed_buffer_ptr(fixed.buffer_id()) {
                    Ok(ptr) => ptr,
                    Err(e) => {
                        error!("handle_write_request(): invalid fixed buffer (error={e:?})");
                        return Ok(build_error(source, ErrorCode::InvalidMessage));
                    },
                };
                fixed_buffer_ptr = Some(ptr);
                fixed.data_len() as usize
            },
            Ok(VenvCommand::Shutdown) => {
                debug!("handle_write_request(): received shutdown while waiting for bulk data");
                return Err(WorkerThreadError::Interrupted);
            },
            Ok(VenvCommand::Work(_)) => {
                error!("handle_write_request(): expected bulk data, got IKC message");
                return Ok(build_error(source, ErrorCode::InvalidMessage));
            },
            Err(e) => {
                error!("handle_write_request(): failed to receive bulk data");
                return Err(e);
            },
        };

        let count: usize = request.count as usize;
        let write_len: usize = core::cmp::min(count, fixed_buffer_len);
        let write_buf: &[u8] = if let Some(ref bulk_data) = bulk_data {
            &bulk_data[..core::cmp::min(count, bulk_data.len())]
        } else {
            let Some(ptr) = fixed_buffer_ptr else {
                error!("handle_write_request(): missing fixed buffer pointer");
                return Ok(build_error(source, ErrorCode::InvalidMessage));
            };
            // SAFETY: `fixed_buffer_ptr` is validated against the shared ring mapping and
            // `write_len` is bounded by the fixed-buffer transfer length.
            unsafe { ::std::slice::from_raw_parts(ptr.cast_const(), write_len) }
        };

        // Check if writing to gateway.
        if request.fd == STDOUT_FILENO || request.fd == STDERR_FILENO {
            // Check if write size is invalid.
            if count == 0 {
                // Writing zero-bytes to STDOUT is not allowed, as we used this to signal EOF.
                error!("handle_write_request(): trying to write zero bytes to STDOUT");
                Ok(build_error(source, ErrorCode::InvalidArgument))
            } else {
                let mut locked_gateway_writer: MutexGuard<'_, SocketStreamWriter> =
                    gateway_writer.blocking_lock();

                // Run blocking write as a cancellable operation.
                let result: ::std::io::Result<()> =
                    Self::run_cancellable_operation(locked_gateway_writer.write_all(write_buf));

                match result {
                    Ok(()) => {
                        debug!("wrote {} bytes to the gateway", write_buf.len());
                        Ok(WriteResponse::build(source, write_buf.len() as i32))
                    },
                    Err(e) if e.kind() == ErrorKind::Interrupted => {
                        debug!("handle_write_request(): write interrupted");
                        Err(WorkerThreadError::Interrupted)
                    },
                    Err(e) => {
                        error!("failed to write to gateway socket (error={e:?})");
                        Ok(build_error(source, ErrorCode::IoErr))
                    },
                }
            }
        } else {
            // Write to other file descriptor via syscall table redirection.
            let fd: libc::c_int = request.fd;
            let ret: libc::ssize_t = unsafe {
                unistd::do_write(
                    syscall_table,
                    fd,
                    write_buf.as_ptr() as *const libc::c_void,
                    count,
                )
            };
            if ret >= 0 {
                Ok(WriteResponse::build(source, ret as i32))
            } else {
                let errno: i32 = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    error!(
                        "handle_write_request(): worker thread interrupted while blocked on \
                         write()"
                    );
                    Err(WorkerThreadError::Interrupted)
                } else {
                    error!(
                        "handle_write_request(): write via syscall table failed (errno={errno})"
                    );
                    Ok(build_error(
                        source,
                        ErrorCode::try_from(errno).unwrap_or_else(|_| {
                            error!(
                                "handle_write_request(): unmapped errno={errno}, falling back to \
                                 IoErr"
                            );
                            ErrorCode::IoErr
                        }),
                    ))
                }
            }
        }
    }

    fn handle_read_request<T>(
        syscall_table: &SyscallTable<T>,
        gateway_reader: Arc<Mutex<SocketStreamReader>>,
        source: ThreadIdentifier,
        request: ReadRequest,
        channel_rx: &mut Receiver<VenvCommand>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        shared_ring: Option<Arc<SharedRing>>,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_read_request(): source={source:?}, request={request:?}");

        enum PullTarget {
            Bulk(::sys::ipc::DataChunkHeader),
            Fixed(FixedBufferTransfer, Arc<SharedRing>),
        }

        let pull_target: PullTarget = match Self::recv_with_timeout(channel_rx, BULK_DATA_TIMEOUT) {
            Ok(VenvCommand::BulkData(bulk)) => PullTarget::Bulk(*bulk.header()),
            Ok(VenvCommand::FixedBuffer(fixed)) => {
                let Some(shared_ring) = shared_ring else {
                    error!(
                        "handle_read_request(): received fixed-buffer transfer without shared ring mapping"
                    );
                    return Ok(build_error(source, ErrorCode::InvalidMessage));
                };
                PullTarget::Fixed(fixed, shared_ring)
            },
            Ok(VenvCommand::Shutdown) => {
                debug!("handle_read_request(): received shutdown while waiting for bulk data");
                return Err(WorkerThreadError::Interrupted);
            },
            Ok(VenvCommand::Work(_)) => {
                error!("handle_read_request(): expected bulk data, got IKC message");
                return Ok(build_error(source, ErrorCode::InvalidMessage));
            },
            Err(e) => {
                error!("handle_read_request(): failed to receive bulk data");
                return Err(e);
            },
        };

        let max_len: usize = match &pull_target {
            PullTarget::Bulk(header) => header.data_len() as usize,
            PullTarget::Fixed(fixed, _) => fixed.data_len() as usize,
        };

        let send_response =
            |data: Option<Vec<u8>>, len: u32, pull_target: &PullTarget| -> Result<(), WorkerThreadError> {
                match pull_target {
                    PullTarget::Bulk(header) => {
                        let bulk: ::sys::ipc::DataChunk = ::sys::ipc::DataChunk::new(
                            ::sys::ipc::DataChunkHeader::new(
                                header.source_pid(),
                                header.source_tid(),
                                header.destination_pid(),
                                header.destination_tid(),
                                header.data_addr(),
                                len,
                            ),
                            data.unwrap_or_default(),
                        );
                        Handle::current()
                            .block_on(Self::send_bulk(uvm_stream.clone(), &bulk))
                            .map_err(|e| {
                                if e.kind() == ErrorKind::BrokenPipe {
                                    debug!("handle_read_request(): UVM stream closed (broken pipe)");
                                    WorkerThreadError::Interrupted
                                } else {
                                    error!(
                                        "handle_read_request(): failed to send bulk response (error={e:?})"
                                    );
                                    WorkerThreadError::Interrupted
                                }
                            })
                    },
                    PullTarget::Fixed(fixed, _) => {
                        let response: FixedBufferTransfer = FixedBufferTransfer::new(
                            fixed.source_pid(),
                            fixed.source_tid(),
                            fixed.destination_pid(),
                            fixed.destination_tid(),
                            fixed.buffer_id(),
                            len,
                        );
                        Handle::current()
                            .block_on(Self::send_fixed(uvm_stream.clone(), &response))
                            .map_err(|e| {
                                if e.kind() == ErrorKind::BrokenPipe {
                                    debug!("handle_read_request(): UVM stream closed (broken pipe)");
                                    WorkerThreadError::Interrupted
                                } else {
                                    error!(
                                        "handle_read_request(): failed to send fixed-buffer response (error={e:?})"
                                    );
                                    WorkerThreadError::Interrupted
                                }
                            })
                    },
                }
            };

        if request.fd == STDIN_FILENO {
            let mut locked_gateway_reader: MutexGuard<'_, SocketStreamReader> =
                gateway_reader.blocking_lock();

            match &pull_target {
                PullTarget::Fixed(fixed, ring) => {
                    let ptr: *mut u8 = match ring.fixed_buffer_ptr(fixed.buffer_id()) {
                        Ok(ptr) => ptr,
                        Err(e) => {
                            error!("handle_read_request(): invalid fixed buffer (error={e:?})");
                            return Ok(build_error(source, ErrorCode::InvalidMessage));
                        },
                    };
                    // SAFETY: `ptr` points to a validated fixed buffer and `max_len` is bounded by
                    // the fixed-buffer transfer length.
                    let read_slice: &mut [u8] =
                        unsafe { ::std::slice::from_raw_parts_mut(ptr, max_len) };
                    let result: ::std::io::Result<usize> =
                        Self::run_cancellable_operation(locked_gateway_reader.read(read_slice));
                    drop(locked_gateway_reader);

                    match result {
                        Ok(0) => {
                            debug!("handle_read_request(): eof");
                            send_response(None, 0, &pull_target)?;
                            Ok(ReadResponse::eof(source))
                        },
                        Ok(n) => {
                            debug!("read {n} bytes from gateway");
                            send_response(None, n as u32, &pull_target)?;
                            let empty_buf: [u8; ReadResponse::BUFFER_SIZE] =
                                [0u8; ReadResponse::BUFFER_SIZE];
                            Ok(ReadResponse::build(source, n as c_ssize_t, empty_buf))
                        },
                        Err(e) if e.kind() == ErrorKind::Interrupted => {
                            debug!("handle_read_request(): read interrupted");
                            if let Err(send_err) = send_response(None, 0, &pull_target) {
                                warn!(
                                    "handle_read_request(): failed to send empty fixed-buffer response on interrupt (error={send_err:?})"
                                );
                            }
                            Err(WorkerThreadError::Interrupted)
                        },
                        Err(e) => {
                            error!("handle_read_request(): error reading data from gateway (error={e:?})");
                            send_response(None, 0, &pull_target)?;
                            Ok(ReadResponse::eof(source))
                        },
                    }
                },
                PullTarget::Bulk(_) => {
                    let mut read_buf: Vec<u8> = ::std::vec![0u8; max_len];
                    let result: ::std::io::Result<usize> =
                        Self::run_cancellable_operation(locked_gateway_reader.read(&mut read_buf));
                    drop(locked_gateway_reader);

                    match result {
                        Ok(0) => {
                            debug!("handle_read_request(): eof");
                            send_response(Some(Vec::new()), 0, &pull_target)?;
                            Ok(ReadResponse::eof(source))
                        },
                        Ok(n) => {
                            debug!("read {n} bytes from gateway");
                            read_buf.truncate(n);
                            profiler::timestamp_message!(&mut read_buf, 0);
                            send_response(Some(read_buf), n as u32, &pull_target)?;
                            let empty_buf: [u8; ReadResponse::BUFFER_SIZE] =
                                [0u8; ReadResponse::BUFFER_SIZE];
                            Ok(ReadResponse::build(source, n as c_ssize_t, empty_buf))
                        },
                        Err(e) if e.kind() == ErrorKind::Interrupted => {
                            debug!("handle_read_request(): read interrupted");
                            if let Err(send_err) = send_response(Some(Vec::new()), 0, &pull_target)
                            {
                                warn!(
                                    "handle_read_request(): failed to send empty bulk response on interrupt (error={send_err:?})"
                                );
                            }
                            Err(WorkerThreadError::Interrupted)
                        },
                        Err(e) => {
                            error!("handle_read_request(): error reading data from gateway (error={e:?})");
                            send_response(Some(Vec::new()), 0, &pull_target)?;
                            Ok(ReadResponse::eof(source))
                        },
                    }
                },
            }
        } else {
            let fd: libc::c_int = request.fd;

            match &pull_target {
                PullTarget::Fixed(fixed, ring) => {
                    let ptr: *mut u8 = match ring.fixed_buffer_ptr(fixed.buffer_id()) {
                        Ok(ptr) => ptr,
                        Err(e) => {
                            error!("handle_read_request(): invalid fixed buffer (error={e:?})");
                            return Ok(build_error(source, ErrorCode::InvalidMessage));
                        },
                    };
                    let ret: libc::ssize_t = unsafe {
                        unistd::do_read(
                            syscall_table,
                            fd,
                            ptr as *mut libc::c_void,
                            max_len,
                        )
                    };
                    if ret > 0 {
                        let n: usize = ret as usize;
                        send_response(None, n as u32, &pull_target)?;
                        let empty_buf: [u8; ReadResponse::BUFFER_SIZE] =
                            [0u8; ReadResponse::BUFFER_SIZE];
                        Ok(ReadResponse::build(source, n as c_ssize_t, empty_buf))
                    } else if ret == 0 {
                        send_response(None, 0, &pull_target)?;
                        Ok(ReadResponse::eof(source))
                    } else {
                        let errno: i32 = unsafe { *libc::__errno_location() };
                        if errno == libc::EINTR {
                            error!(
                                "handle_read_request(): worker thread interrupted while blocked on read()"
                            );
                            if let Err(send_err) = send_response(None, 0, &pull_target) {
                                warn!(
                                    "handle_read_request(): failed to send empty fixed-buffer response on interrupt (error={send_err:?})"
                                );
                            }
                            Err(WorkerThreadError::Interrupted)
                        } else {
                            error!("handle_read_request(): read via syscall table failed (errno={errno})");
                            send_response(None, 0, &pull_target)?;
                            Ok(build_error(
                                source,
                                ErrorCode::try_from(errno).unwrap_or_else(|_| {
                                    error!(
                                        "handle_read_request(): unmapped errno={errno}, falling back to IoErr"
                                    );
                                    ErrorCode::IoErr
                                }),
                            ))
                        }
                    }
                },
                PullTarget::Bulk(_) => {
                    let mut read_buf: Vec<u8> = ::std::vec![0u8; max_len];
                    let ret: libc::ssize_t = unsafe {
                        unistd::do_read(
                            syscall_table,
                            fd,
                            read_buf.as_mut_ptr() as *mut libc::c_void,
                            max_len,
                        )
                    };
                    if ret > 0 {
                        let n: usize = ret as usize;
                        read_buf.truncate(n);
                        send_response(Some(read_buf), n as u32, &pull_target)?;
                        let empty_buf: [u8; ReadResponse::BUFFER_SIZE] =
                            [0u8; ReadResponse::BUFFER_SIZE];
                        Ok(ReadResponse::build(source, n as c_ssize_t, empty_buf))
                    } else if ret == 0 {
                        send_response(Some(Vec::new()), 0, &pull_target)?;
                        Ok(ReadResponse::eof(source))
                    } else {
                        let errno: i32 = unsafe { *libc::__errno_location() };
                        if errno == libc::EINTR {
                            error!(
                                "handle_read_request(): worker thread interrupted while blocked on read()"
                            );
                            if let Err(send_err) = send_response(Some(Vec::new()), 0, &pull_target)
                            {
                                warn!(
                                    "handle_read_request(): failed to send empty bulk response on interrupt (error={send_err:?})"
                                );
                            }
                            Err(WorkerThreadError::Interrupted)
                        } else {
                            error!("handle_read_request(): read via syscall table failed (errno={errno})");
                            send_response(Some(Vec::new()), 0, &pull_target)?;
                            Ok(build_error(
                                source,
                                ErrorCode::try_from(errno).unwrap_or_else(|_| {
                                    error!(
                                        "handle_read_request(): unmapped errno={errno}, falling back to IoErr"
                                    );
                                    ErrorCode::IoErr
                                }),
                            ))
                        }
                    }
                },
            }
        }
    }

    fn handle_positioned_write_request<T>(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: PositionedWriteRequest,
        channel_rx: &mut Receiver<VenvCommand>,
        shared_ring: Option<Arc<SharedRing>>,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_positioned_write_request(): source={source:?}, request={request:?}");

        let mut bulk_data: Option<Vec<u8>> = None;
        let mut fixed_buffer_ptr: Option<*mut u8> = None;
        let fixed_buffer_len: usize = match Self::recv_with_timeout(channel_rx, BULK_DATA_TIMEOUT) {
            Ok(VenvCommand::BulkData(bulk)) => {
                let mut data: Vec<u8> = bulk.into_data();
                profiler::timestamp_message!(&mut data, 0);
                let len: usize = data.len();
                bulk_data = Some(data);
                len
            },
            Ok(VenvCommand::FixedBuffer(fixed)) => {
                let Some(shared_ring) = shared_ring else {
                    error!(
                        "handle_positioned_write_request(): received fixed-buffer transfer without shared ring mapping"
                    );
                    return Ok(build_error(source, ErrorCode::InvalidMessage));
                };
                let ptr: *mut u8 = match shared_ring.fixed_buffer_ptr(fixed.buffer_id()) {
                    Ok(ptr) => ptr,
                    Err(e) => {
                        error!(
                            "handle_positioned_write_request(): invalid fixed buffer (error={e:?})"
                        );
                        return Ok(build_error(source, ErrorCode::InvalidMessage));
                    },
                };
                fixed_buffer_ptr = Some(ptr);
                fixed.data_len() as usize
            },
            Ok(VenvCommand::Shutdown) => {
                debug!(
                    "handle_positioned_write_request(): received shutdown while waiting for bulk data"
                );
                return Err(WorkerThreadError::Interrupted);
            },
            Ok(VenvCommand::Work(_)) => {
                error!("handle_positioned_write_request(): expected bulk data, got IKC message");
                return Ok(build_error(source, ErrorCode::InvalidMessage));
            },
            Err(e) => {
                error!("handle_positioned_write_request(): failed to receive bulk data");
                return Err(e);
            },
        };

        let fd: libc::c_int = request.fd;
        let count: usize = request.count as usize;
        let offset: libc::off_t = request.offset;
        let write_buf: &[u8] = if let Some(ref bulk_data) = bulk_data {
            &bulk_data[..core::cmp::min(count, bulk_data.len())]
        } else {
            let Some(ptr) = fixed_buffer_ptr else {
                error!("handle_positioned_write_request(): missing fixed buffer pointer");
                return Ok(build_error(source, ErrorCode::InvalidMessage));
            };
            let write_len: usize = core::cmp::min(count, fixed_buffer_len);
            // SAFETY: `ptr` points to a validated fixed buffer and `write_len` is bounded by the
            // transfer length announced by the guest.
            unsafe { ::std::slice::from_raw_parts(ptr.cast_const(), write_len) }
        };

        let ret: libc::ssize_t = unsafe {
            unistd::do_pwrite_raw(
                syscall_table,
                fd,
                write_buf.as_ptr() as *const libc::c_void,
                count,
                offset,
            )
        };
        if ret >= 0 {
            Ok(WriteResponse::build(source, ret as c_ssize_t))
        } else {
            let errno: i32 = unsafe { *libc::__errno_location() };
            if errno == libc::EINTR {
                error!(
                    "handle_positioned_write_request(): worker thread interrupted while blocked on \
                     pwrite()"
                );
                Err(WorkerThreadError::Interrupted)
            } else {
                error!(
                    "handle_positioned_write_request(): pwrite via syscall table failed (errno={errno})"
                );
                Ok(build_error(
                    source,
                    ErrorCode::try_from(errno).unwrap_or_else(|_| {
                        error!(
                            "handle_positioned_write_request(): unmapped errno={errno}, falling back \
                             to IoErr"
                        );
                        ErrorCode::IoErr
                    }),
                ))
            }
        }
    }

    fn handle_positioned_read_request<T>(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: PositionedReadRequest,
        channel_rx: &mut Receiver<VenvCommand>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        shared_ring: Option<Arc<SharedRing>>,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_positioned_read_request(): source={source:?}, request={request:?}");

        enum PullTarget {
            Bulk(::sys::ipc::DataChunkHeader),
            Fixed(FixedBufferTransfer, Arc<SharedRing>),
        }

        let pull_target: PullTarget = match Self::recv_with_timeout(channel_rx, BULK_DATA_TIMEOUT) {
            Ok(VenvCommand::BulkData(bulk)) => PullTarget::Bulk(*bulk.header()),
            Ok(VenvCommand::FixedBuffer(fixed)) => {
                let Some(shared_ring) = shared_ring else {
                    error!(
                        "handle_positioned_read_request(): received fixed-buffer transfer without shared ring mapping"
                    );
                    return Ok(build_error(source, ErrorCode::InvalidMessage));
                };
                PullTarget::Fixed(fixed, shared_ring)
            },
            Ok(VenvCommand::Shutdown) => {
                debug!(
                    "handle_positioned_read_request(): received shutdown while waiting for bulk data"
                );
                return Err(WorkerThreadError::Interrupted);
            },
            Ok(VenvCommand::Work(_)) => {
                error!("handle_positioned_read_request(): expected BulkData, got IKC message");
                return Ok(build_error(source, ErrorCode::InvalidMessage));
            },
            Err(e) => {
                error!("handle_positioned_read_request(): failed to receive bulk data");
                return Err(e);
            },
        };

        let max_len: usize = match &pull_target {
            PullTarget::Bulk(header) => header.data_len() as usize,
            PullTarget::Fixed(fixed, _) => fixed.data_len() as usize,
        };
        let offset: libc::off_t = request.offset;

        let send_response =
            |data: Option<Vec<u8>>, len: u32, pull_target: &PullTarget| -> Result<(), WorkerThreadError> {
                match pull_target {
                    PullTarget::Bulk(header) => {
                        let bulk: ::sys::ipc::DataChunk = ::sys::ipc::DataChunk::new(
                            ::sys::ipc::DataChunkHeader::new(
                                header.source_pid(),
                                header.source_tid(),
                                header.destination_pid(),
                                header.destination_tid(),
                                header.data_addr(),
                                len,
                            ),
                            data.unwrap_or_default(),
                        );
                        Handle::current()
                            .block_on(Self::send_bulk(uvm_stream.clone(), &bulk))
                            .map_err(|e| {
                                if e.kind() == ErrorKind::BrokenPipe {
                                    debug!(
                                        "handle_positioned_read_request(): UVM stream closed (broken pipe)"
                                    );
                                    WorkerThreadError::Interrupted
                                } else {
                                    error!(
                                        "handle_positioned_read_request(): failed to send bulk response (error={e:?})"
                                    );
                                    WorkerThreadError::Interrupted
                                }
                            })
                    },
                    PullTarget::Fixed(fixed, _) => {
                        let response: FixedBufferTransfer = FixedBufferTransfer::new(
                            fixed.source_pid(),
                            fixed.source_tid(),
                            fixed.destination_pid(),
                            fixed.destination_tid(),
                            fixed.buffer_id(),
                            len,
                        );
                        Handle::current()
                            .block_on(Self::send_fixed(uvm_stream.clone(), &response))
                            .map_err(|e| {
                                if e.kind() == ErrorKind::BrokenPipe {
                                    debug!(
                                        "handle_positioned_read_request(): UVM stream closed (broken pipe)"
                                    );
                                    WorkerThreadError::Interrupted
                                } else {
                                    error!(
                                        "handle_positioned_read_request(): failed to send fixed-buffer response (error={e:?})"
                                    );
                                    WorkerThreadError::Interrupted
                                }
                            })
                    },
                }
            };

        let fd: libc::c_int = request.fd;
        match &pull_target {
            PullTarget::Fixed(fixed, ring) => {
                let ptr: *mut u8 = match ring.fixed_buffer_ptr(fixed.buffer_id()) {
                    Ok(ptr) => ptr,
                    Err(e) => {
                        error!(
                            "handle_positioned_read_request(): invalid fixed buffer (error={e:?})"
                        );
                        return Ok(build_error(source, ErrorCode::InvalidMessage));
                    },
                };
                let ret: libc::ssize_t = unsafe {
                    unistd::do_pread_raw(
                        syscall_table,
                        fd,
                        ptr as *mut libc::c_void,
                        max_len,
                        offset,
                    )
                };
                if ret > 0 {
                    let n: usize = ret as usize;
                    send_response(None, n as u32, &pull_target)?;
                    let empty_buf: [u8; ReadResponse::BUFFER_SIZE] =
                        [0u8; ReadResponse::BUFFER_SIZE];
                    Ok(ReadResponse::build(source, n as c_ssize_t, empty_buf))
                } else if ret == 0 {
                    send_response(None, 0, &pull_target)?;
                    Ok(ReadResponse::eof(source))
                } else {
                    let errno: i32 = unsafe { *libc::__errno_location() };
                    if errno == libc::EINTR {
                        error!(
                            "handle_positioned_read_request(): worker thread interrupted while blocked on pread()"
                        );
                        if let Err(send_err) = send_response(None, 0, &pull_target) {
                            warn!(
                                "handle_positioned_read_request(): failed to send empty fixed-buffer response on interrupt (error={send_err:?})"
                            );
                        }
                        Err(WorkerThreadError::Interrupted)
                    } else {
                        error!(
                            "handle_positioned_read_request(): pread via syscall table failed (errno={errno})"
                        );
                        send_response(None, 0, &pull_target)?;
                        Ok(build_error(
                            source,
                            ErrorCode::try_from(errno).unwrap_or_else(|_| {
                                error!(
                                    "handle_positioned_read_request(): unmapped errno={errno}, falling back to IoErr"
                                );
                                ErrorCode::IoErr
                            }),
                        ))
                    }
                }
            },
            PullTarget::Bulk(_) => {
                let mut read_buf: Vec<u8> = ::std::vec![0u8; max_len];
                let ret: libc::ssize_t = unsafe {
                    unistd::do_pread_raw(
                        syscall_table,
                        fd,
                        read_buf.as_mut_ptr() as *mut libc::c_void,
                        max_len,
                        offset,
                    )
                };
                if ret > 0 {
                    let n: usize = ret as usize;
                    read_buf.truncate(n);
                    send_response(Some(read_buf), n as u32, &pull_target)?;
                    let empty_buf: [u8; ReadResponse::BUFFER_SIZE] =
                        [0u8; ReadResponse::BUFFER_SIZE];
                    Ok(ReadResponse::build(source, n as c_ssize_t, empty_buf))
                } else if ret == 0 {
                    send_response(Some(Vec::new()), 0, &pull_target)?;
                    Ok(ReadResponse::eof(source))
                } else {
                    let errno: i32 = unsafe { *libc::__errno_location() };
                    if errno == libc::EINTR {
                        error!(
                            "handle_positioned_read_request(): worker thread interrupted while blocked on pread()"
                        );
                        if let Err(send_err) = send_response(Some(Vec::new()), 0, &pull_target) {
                            warn!(
                                "handle_positioned_read_request(): failed to send empty bulk response on interrupt (error={send_err:?})"
                            );
                        }
                        Err(WorkerThreadError::Interrupted)
                    } else {
                        error!(
                            "handle_positioned_read_request(): pread via syscall table failed (errno={errno})"
                        );
                        send_response(Some(Vec::new()), 0, &pull_target)?;
                        Ok(build_error(
                            source,
                            ErrorCode::try_from(errno).unwrap_or_else(|_| {
                                error!(
                                    "handle_positioned_read_request(): unmapped errno={errno}, falling back to IoErr"
                                );
                                ErrorCode::IoErr
                            }),
                        ))
                    }
                }
            },
        }
    }

    fn handle_fstat_request<T>(
        syscall_table: &SyscallTable<T>,
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

    fn handle_getcwd_request<T>(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        syscall_table: &SyscallTable<T>,
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

    fn handle_getdents_request<T>(
        syscall_table: &SyscallTable<T>,
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

    fn handle_long_request<S, T>(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        syscall_table: &SyscallTable<S>,
        source: ThreadIdentifier,
        message: &LinuxDaemonMessage,
    ) -> Result<(), WorkerThreadError>
    where
        T: RequestAssemblerTrait<S>,
    {
        let part: LinuxDaemonMessagePart = LinuxDaemonMessagePart::from_bytes(message.payload);

        trace!("handle_long_request(): source={source:?}, part={part:?}");

        // Phase 1: Assemble message parts under the lock.  The lock is
        // released as soon as assembly completes (before any blocking
        // syscall runs) so that other worker threads are not starved.
        let assembled_request: Result<Option<T>, WorkerThreadError> = assembler
            .blocking_lock()
            .assemble_and_take::<S, T>(source, part);

        // Phase 2: Process the assembled request *outside* the lock.
        match assembled_request {
            Ok(Some(request)) => {
                let result: Result<Vec<Message>, WorkerThreadError> =
                    T::process_request(syscall_table, source, request);
                match result {
                    Ok(messages) => {
                        for message in messages {
                            if let Err(e) =
                                Handle::current().block_on(Self::send(uvm_stream.clone(), message))
                            {
                                error!("failed to send message (error={e:?})");
                            }
                        }
                        Ok(())
                    },
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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Signal handler for worker thread cancellation.
///
/// This handler is invoked when a registered signal is received by a worker thread. It sets the
/// thread-local cancellation flag, which causes any ongoing async I/O operations to be interrupted
/// and return `ErrorKind::Interrupted`.
///
/// # Safety
///
/// This function uses only async-signal-safe operations:
/// - `AtomicBool::store()` is async-signal-safe as it performs a simple atomic write.
/// - Thread-local storage access is async-signal-safe.
/// - No heap allocations, locks, or other unsafe operations are performed.
///
extern "C" fn linuxd_worker_thread_signal_handler(_: i32) {
    CANCELLATION_FLAG.with(|flag| {
        flag.store(true, Ordering::Relaxed);
    });
}
