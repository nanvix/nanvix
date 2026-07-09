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
        sys_socket,
        sys_times,
        unistd,
    },
    message::{
        RequestAssembler,
        RequestAssemblerTrait,
    },
    syscalls::SyscallTable,
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
    message::SystemCallMessagePart,
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
    SystemCallMessage,
    SystemCallMessageHeader,
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
        watch,
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

/// Signal used to interrupt blocking libc syscalls in worker threads.
const INTERRUPT_SIGNAL: c_int = SIGUSR1;

/// Maximum time a worker thread will wait for the bulk data message that must follow
/// a `ReadRequest` or `WriteRequest`. If the guest VM crashes (or the channel stalls) after
/// sending the IKC request but before the corresponding push/pull arrives, this timeout prevents
/// the worker thread from blocking forever.
const BULK_DATA_TIMEOUT: Duration = Duration::from_secs(30);

//==================================================================================================
// Structures
//==================================================================================================

/// State associated with a worker thread in linuxd.
pub struct WorkerThreadHandle {
    /// Internal thread identifier in linuxd.
    pub id: ThreadIdentifier,
    /// Underlying pthread id, used to deliver `INTERRUPT_SIGNAL` for blocking libc syscalls.
    pthread_id: Arc<AtomicUsize>,
    /// Join handle for the underlying tokio blocking task.
    pub handle: JoinHandle<()>,
    /// Handle to send shutdown messages to message queue.
    pub cmd_tx: Sender<VenvCommand>,
    /// Sender half of the cancellation watch channel.  Sending `true` triggers cancellation of
    /// any in-flight `run_cancellable_operation` or `recv_with_timeout` on the worker thread.
    cancel_tx: watch::Sender<bool>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl WorkerThreadHandle {
    ///
    /// # Description
    ///
    /// Installs a no-op signal handler for `INTERRUPT_SIGNAL` so that blocking libc syscalls
    /// return `EINTR` instead of terminating the process.  The handler deliberately omits
    /// `SA_RESTART` so interrupted syscalls are **not** automatically restarted.
    ///
    fn install_signal_handler() {
        // SAFETY: We install a trivial no-op handler; the only side-effect is that blocking
        // syscalls on this thread will return EINTR when the signal is delivered.
        let ret: c_int = unsafe {
            let sig_action = sigaction {
                sa_sigaction: linuxd_worker_thread_signal_handler as *const () as usize,
                sa_mask: {
                    let mut set = mem::zeroed();
                    sigemptyset(&mut set);
                    set
                },
                sa_flags: 0,
                sa_restorer: None,
            };

            sigaction(INTERRUPT_SIGNAL, &sig_action, ptr::null_mut())
        };

        if ret != 0 {
            let errno: i32 = unsafe { *libc::__errno_location() };
            error!("error installing signal handler (errno={errno:?})");
        }
    }

    ///
    /// # Description
    ///
    /// Run an async operation with cancellation support via a watch channel.
    ///
    /// This helper function wraps an async operation to make it interruptible via the
    /// cancellation watch channel. When `stop()` is called on the worker thread handle,
    /// the watch value changes to `true` and the operation is cancelled immediately,
    /// returning `ErrorKind::Interrupted`.
    ///
    /// # Parameters
    ///
    /// * `f` - The async operation to run. Must return a `std::io::Result<R>`.
    /// * `cancel_rx` - The watch receiver used to detect cancellation.
    ///
    /// # Returns
    ///
    /// Returns the result of the async operation, or an `Interrupted` error if cancelled.
    ///
    /// # Cancel Safety
    ///
    /// When `cancel_rx.wait_for()` wins the `select!`, the future `f` is dropped. The callers
    /// pass tokio socket read/write futures which are cancel-safe, so no data is lost.
    ///
    fn run_cancellable_operation<F, R>(
        f: F,
        cancel_rx: &mut watch::Receiver<bool>,
    ) -> ::std::io::Result<R>
    where
        F: ::std::future::Future<Output = ::std::io::Result<R>>,
    {
        Handle::current().block_on(async {
            ::tokio::pin!(f);

            ::tokio::select! {
                result = &mut f => result,
                // `wait_for` resolves immediately if the value is already `true`,
                // so a cancellation that arrived before this call is not missed.
                _ = cancel_rx.wait_for(|v| *v) => {
                    Err(::std::io::Error::new(
                        ErrorKind::Interrupted,
                        "run_cancellable_operation(): operation cancelled",
                    ))
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
    /// - `cancel_rx`: Watch receiver used to detect cancellation.
    ///
    /// # Returns
    ///
    /// The received command, or a [`WorkerThreadError`] if the operation times out, the channel
    /// closes, or the thread is cancelled.
    ///
    fn recv_with_timeout(
        channel_rx: &mut Receiver<VenvCommand>,
        timeout: Duration,
        cancel_rx: &mut watch::Receiver<bool>,
    ) -> Result<VenvCommand, WorkerThreadError> {
        Handle::current().block_on(async {
            ::tokio::select! {
                result = ::tokio::time::timeout(timeout, channel_rx.recv()) => {
                    match result {
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
                }
                // `wait_for` resolves immediately if the value is already `true`,
                // so a cancellation that arrived before this call is not missed.
                _ = cancel_rx.wait_for(|v| *v) => {
                    debug!("recv_with_timeout(): cancelled");
                    Err(WorkerThreadError::Interrupted)
                }
            }
        })
    }

    ///
    /// # Description
    ///
    /// Spawns an interruptible worker thread that processes [`VenvCommand`]s from the given
    /// channel.  The thread installs a signal handler and runs the main message loop until a
    /// shutdown command or cancellation is received.
    ///
    /// # Parameters
    ///
    /// - `id`: Internal thread identifier in linuxd.
    /// - `channel_rx`: Receiver end of the command channel.
    /// - `channel_tx`: Sender end retained in the handle for shutdown delivery.
    /// - `uvm_handle`: Handle to the user VM connection.
    /// - `assembler`: Shared request assembler for multi-part messages.
    /// - `syscall_table`: Shared syscall dispatch table.
    ///
    /// # Returns
    ///
    /// A [`WorkerThreadHandle`] on success, or an error if spawning fails.
    ///
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

        // Create a watch channel for cancellation.  The initial value `false` means "not
        // cancelled".  Sending `true` triggers immediate cancellation of any in-flight
        // cancellable operation.
        let (cancel_tx, cancel_rx) = watch::channel(false);

        // Atomic holder for the pthread id so `stop()` can send a signal.
        let pthread_id_holder: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let pthread_id_clone: Arc<AtomicUsize> = pthread_id_holder.clone();

        let join_handle = task::spawn_blocking(move || {
            // SAFETY: `pthread_self()` returns the calling thread's id.
            let pthread_id: libc::pthread_t = unsafe { pthread_self() };
            pthread_id_holder.store(pthread_id as usize, Ordering::Release);

            Self::install_signal_handler();

            Self::handle_message(channel_rx, uvm_handle, syscall_table, assembler, cancel_rx);

            trace!("thread shutting down (pthread_id={pthread_id})");
        });

        Ok(Self {
            id,
            pthread_id: pthread_id_clone,
            handle: join_handle,
            cmd_tx: channel_tx,
            cancel_tx,
        })
    }

    ///
    /// # Description
    ///
    /// Stops a worker thread by triggering its cancellation watch channel and sending
    /// `INTERRUPT_SIGNAL` to interrupt any blocking libc syscall.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if the cancellation channel is already closed.
    ///
    pub fn stop(&self) -> Result<(), Error> {
        // Trigger the watch channel first so that any tokio `select!` branch sees
        // the cancellation immediately.
        self.cancel_tx.send(true).map_err(|_| {
            let reason: &str = "cancellation channel closed";
            error!("{reason}");
            Error::new(ErrorCode::InvalidArgument, reason)
        })?;

        // Also send a signal to interrupt blocking libc syscalls (e.g. read/write on
        // non-gateway file descriptors) so they return EINTR.
        let raw_tid: usize = self.pthread_id.load(Ordering::Acquire);
        if raw_tid != 0 {
            // SAFETY: `raw_tid` is a valid `pthread_t` stored by the worker thread at
            // startup.  The thread is guaranteed to be alive because we hold a reference
            // to its `JoinHandle`.  `INTERRUPT_SIGNAL` has a no-op handler installed.
            unsafe { pthread_kill(raw_tid as libc::pthread_t, INTERRUPT_SIGNAL) };
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Main event loop for a worker thread.  Receives commands from the channel, dispatches
    /// them to the appropriate handler, and sends responses back to the user VM.  The loop
    /// exits on a `Shutdown` command, cancellation, or a fatal error.
    ///
    /// # Parameters
    ///
    /// - `channel_rx`: Receiver end of the command channel.
    /// - `uvm_handle`: Handle to the user VM connection.
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `assembler`: Shared request assembler for multi-part messages.
    /// - `cancel_rx`: Watch receiver used to detect cancellation.
    ///
    fn handle_message<T>(
        mut channel_rx: Receiver<VenvCommand>,
        uvm_handle: UserVmHandle,
        syscall_table: Arc<SyscallTable<T>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        mut cancel_rx: watch::Receiver<bool>,
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
            // NOTE: `blocking_recv()` is intentionally not wrapped with `cancel_rx` here.
            // During shutdown, `stop()` triggers the watch channel and then the caller enqueues a
            // `Shutdown` command.  Because `stop()` runs first, any worker blocked in a
            // cancellable I/O or `recv_with_timeout` is unblocked and starts draining the
            // channel, so the bounded channel is guaranteed to have capacity for the `Shutdown`
            // command that wakes this `blocking_recv()`.
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
            // The kernel stamps the originating thread into `message.source.tid`; messages in this
            // channel always address a thread. A `NONE` sentinel names no specific thread, so skip
            // it rather than keying reply routing on the sentinel.
            let source: ThreadIdentifier = { message.source }.tid;
            if source.is_none() {
                error!(
                    "handle_message(): received message with no originating thread, skipping \
                     (worker_tid={worker_tid:?})"
                );
                continue;
            }

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
                sys::ipc::MessageType::ProcessCreationEvent => {
                    error!(
                        "handle_message(): received unexpected process creation event, stopping"
                    );
                    break;
                },
                sys::ipc::MessageType::PullResponse => {
                    error!("handle_message(): received unexpected pull response, stopping");
                    break;
                },
                sys::ipc::MessageType::Ikc => {
                    match SystemCallMessage::try_from_bytes(message.payload) {
                        Ok(message) => {
                            let message: Message = match message.header {
                                // The system calls are interposed before being forwarded to the
                                // backend provider.
                                SystemCallMessageHeader::CloseRequest
                                | SystemCallMessageHeader::ReadRequest
                                | SystemCallMessageHeader::ReceiveSocketRequest
                                | SystemCallMessageHeader::SendSocketRequest
                                | SystemCallMessageHeader::WriteRequest => {
                                    match Self::handle_special_messages(
                                        &syscall_table,
                                        gateway_reader.clone(),
                                        gateway_writer.clone(),
                                        source,
                                        message,
                                        &mut channel_rx,
                                        uvm_stream.clone(),
                                        &mut cancel_rx,
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
                                SystemCallMessageHeader::AcceptSocketRequest
                                | SystemCallMessageHeader::BindSocketRequest
                                | SystemCallMessageHeader::ConnectSocketRequest
                                | SystemCallMessageHeader::CreateSocketPairRequest
                                | SystemCallMessageHeader::CreateSocketRequest
                                | SystemCallMessageHeader::FileAdvisoryInformationRequest
                                | SystemCallMessageHeader::FileChdirRequest
                                | SystemCallMessageHeader::FileChmodRequest
                                | SystemCallMessageHeader::FileChownRequest
                                | SystemCallMessageHeader::FileControlRequest
                                | SystemCallMessageHeader::FileDataSyncRequest
                                | SystemCallMessageHeader::FileSpaceControlRequest
                                | SystemCallMessageHeader::FileSyncRequest
                                | SystemCallMessageHeader::FileTruncateRequest
                                | SystemCallMessageHeader::GetIdsRequest
                                | SystemCallMessageHeader::GetPeerNameRequest
                                | SystemCallMessageHeader::GetSockNameRequest
                                | SystemCallMessageHeader::ListenSocketRequest
                                | SystemCallMessageHeader::PartialReadRequest
                                | SystemCallMessageHeader::PartialWriteRequest
                                | SystemCallMessageHeader::SeekRequest
                                | SystemCallMessageHeader::ShutdownSocketRequest
                                | SystemCallMessageHeader::TimesRequest
                                | SystemCallMessageHeader::PipeRequest
                                | SystemCallMessageHeader::UpdateFileAccessTimeRequest => {
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
                                SystemCallMessageHeader::FileStatRequest
                                | SystemCallMessageHeader::GetCurrentWorkingDirectoryRequest
                                | SystemCallMessageHeader::GetDirectoryEntriesRequest => {
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
                                SystemCallMessageHeader::ChangeDirectoryRequestPart
                                | SystemCallMessageHeader::FileStatAtRequestPart
                                | SystemCallMessageHeader::FileAccessAtRequestPart
                                | SystemCallMessageHeader::SymbolicLinkAtRequestPart
                                | SystemCallMessageHeader::LinkAtRequestPart
                                | SystemCallMessageHeader::ReadLinkAtRequestPart
                                | SystemCallMessageHeader::MakeDirectoryAtRequestPart
                                | SystemCallMessageHeader::UpdateFileAccessTimeAtRequestPart
                                | SystemCallMessageHeader::FileChownAtRequestPart
                                | SystemCallMessageHeader::FileChmodAtRequestPart
                                | SystemCallMessageHeader::OpenAtRequestPart
                                | SystemCallMessageHeader::RenameAtRequestPart
                                | SystemCallMessageHeader::UnlinkAtRequestPart
                                | SystemCallMessageHeader::PollRequestPart
                                | SystemCallMessageHeader::SelectRequestPart => {
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
                            error!("failed to parse system call message (error={e:?})");
                        },
                    }
                },
            }
        }
    }

    ///
    /// # Description
    ///
    /// Dispatches close, read, and write requests to their specialized handlers.  These
    /// requests require interposition (gateway I/O or bulk data transfer) before being
    /// forwarded to the backend syscall provider.
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `gateway_reader`: Shared reader stream for the gateway socket.
    /// - `gateway_writer`: Shared writer stream for the gateway socket.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `message`: The parsed linuxd daemon message.
    /// - `channel_rx`: Worker command channel receiver (for bulk data).
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `cancel_rx`: Watch receiver used to detect cancellation.
    ///
    /// # Returns
    ///
    /// The response [`Message`] on success, or a [`WorkerThreadError`] on failure.
    ///
    #[allow(clippy::too_many_arguments)]
    fn handle_special_messages<T>(
        syscall_table: &SyscallTable<T>,
        gateway_reader: Arc<Mutex<SocketStreamReader>>,
        gateway_writer: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        message: SystemCallMessage,
        channel_rx: &mut Receiver<VenvCommand>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        cancel_rx: &mut watch::Receiver<bool>,
    ) -> Result<Message, WorkerThreadError> {
        match message.header {
            SystemCallMessageHeader::CloseRequest => {
                let request: CloseRequest = CloseRequest::from_bytes(message.payload);
                Self::handle_close_request(syscall_table, source, request)
            },
            SystemCallMessageHeader::ReadRequest => {
                let request: ReadRequest = ReadRequest::from_bytes(message.payload);
                Self::handle_read_request(
                    syscall_table,
                    gateway_reader,
                    source,
                    request,
                    channel_rx,
                    uvm_stream,
                    cancel_rx,
                )
            },
            SystemCallMessageHeader::WriteRequest => {
                let request: WriteRequest = WriteRequest::from_bytes(message.payload);
                Self::handle_write_request(
                    syscall_table,
                    gateway_writer,
                    source,
                    request,
                    channel_rx,
                    cancel_rx,
                )
            },
            SystemCallMessageHeader::ReceiveSocketRequest => {
                let request: ReceiveSocketRequest =
                    ReceiveSocketRequest::from_bytes(message.payload);
                Self::handle_recv_request(
                    syscall_table,
                    source,
                    request,
                    channel_rx,
                    uvm_stream,
                    cancel_rx,
                )
            },
            SystemCallMessageHeader::SendSocketRequest => {
                let request: SendSocketRequest = SendSocketRequest::from_bytes(message.payload);
                Self::handle_send_request(syscall_table, source, request, channel_rx, cancel_rx)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected special message {:?}", header)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Dispatches syscall requests whose request and response data both fit in a single message.
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `message`: The parsed linuxd daemon message.
    ///
    /// # Returns
    ///
    /// The response [`Message`] on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_short_request_messages<T>(
        syscall_table: Arc<SyscallTable<T>>,
        source: ThreadIdentifier,
        message: SystemCallMessage,
    ) -> Result<Message, WorkerThreadError> {
        match message.header {
            SystemCallMessageHeader::AcceptSocketRequest => {
                let request: AcceptSocketRequest = AcceptSocketRequest::from_bytes(message.payload);
                sys_socket::do_accept(&syscall_table, source, request)
            },
            SystemCallMessageHeader::BindSocketRequest => {
                let request: BindSocketRequest = BindSocketRequest::from_bytes(message.payload);
                sys_socket::do_bind(&syscall_table, source, request)
            },
            SystemCallMessageHeader::ConnectSocketRequest => {
                let request: ConnectSocketRequest =
                    ConnectSocketRequest::from_bytes(message.payload);
                sys_socket::do_connect(&syscall_table, source, request)
            },
            SystemCallMessageHeader::CreateSocketPairRequest => {
                let request: CreateSocketPairRequest =
                    CreateSocketPairRequest::from_bytes(message.payload);
                sys_socket::do_socketpair(&syscall_table, source, request)
            },
            SystemCallMessageHeader::CreateSocketRequest => {
                let request: CreateSocketRequest = CreateSocketRequest::from_bytes(message.payload);
                sys_socket::do_socket(&syscall_table, source, request)
            },
            SystemCallMessageHeader::FileAdvisoryInformationRequest => {
                let request: FileAdvisoryInformationRequest =
                    FileAdvisoryInformationRequest::from_bytes(message.payload);
                fcntl::do_posix_fadvise(&syscall_table, source, request)
            },
            SystemCallMessageHeader::FileChdirRequest => {
                let request: FileChdirRequest = FileChdirRequest::from_bytes(message.payload);
                unistd::do_fchdir(&syscall_table, source, request)
            },
            SystemCallMessageHeader::FileChmodRequest => {
                let request: FileChmodRequest = FileChmodRequest::from_bytes(message.payload);
                fcntl::do_fchmod(&syscall_table, source, request)
            },
            SystemCallMessageHeader::FileChownRequest => {
                let request: FileChownRequest = FileChownRequest::from_bytes(message.payload);
                unistd::do_fchown(&syscall_table, source, request)
            },
            SystemCallMessageHeader::FileControlRequest => {
                let request: FileControlRequest = FileControlRequest::from_bytes(message.payload);
                fcntl::do_fcntl(&syscall_table, source, request)
            },
            SystemCallMessageHeader::FileDataSyncRequest => {
                let request: FileDataSyncRequest = FileDataSyncRequest::from_bytes(message.payload);
                unistd::do_fdatasync(&syscall_table, source, request)
            },
            SystemCallMessageHeader::FileSpaceControlRequest => {
                let request: FileSpaceControlRequest =
                    FileSpaceControlRequest::from_bytes(message.payload);
                fcntl::do_posix_fallocate(&syscall_table, source, request)
            },
            SystemCallMessageHeader::FileSyncRequest => {
                let request: FileSyncRequest = FileSyncRequest::from_bytes(message.payload);
                unistd::do_fsync(&syscall_table, source, request)
            },
            SystemCallMessageHeader::FileTruncateRequest => {
                let request: FileTruncateRequest = FileTruncateRequest::from_bytes(message.payload);
                unistd::do_ftruncate(&syscall_table, source, request)
            },
            SystemCallMessageHeader::GetIdsRequest => {
                let request: GetIdsRequest = GetIdsRequest::from_bytes(message.payload);
                unistd::do_getids(&syscall_table, source, request)
            },
            SystemCallMessageHeader::GetPeerNameRequest => {
                let request: GetPeerNameRequest = GetPeerNameRequest::from_bytes(message.payload);
                sys_socket::do_getpeername(&syscall_table, source, request)
            },
            SystemCallMessageHeader::GetSockNameRequest => {
                let request: GetSockNameRequest = GetSockNameRequest::from_bytes(message.payload);
                sys_socket::do_getsockname(&syscall_table, source, request)
            },
            SystemCallMessageHeader::ListenSocketRequest => {
                let request: ListenSocketRequest = ListenSocketRequest::from_bytes(message.payload);
                sys_socket::do_listen(&syscall_table, source, request)
            },
            SystemCallMessageHeader::PartialReadRequest => {
                let request: PartialReadRequest = PartialReadRequest::from_bytes(message.payload);
                unistd::do_pread(&syscall_table, source, request)
            },
            SystemCallMessageHeader::PartialWriteRequest => {
                let request: PartialWriteRequest = PartialWriteRequest::from_bytes(message.payload);
                unistd::do_pwrite(&syscall_table, source, request)
            },
            SystemCallMessageHeader::SeekRequest => {
                let request: SeekRequest = SeekRequest::from_bytes(message.payload);
                unistd::do_lseek(&syscall_table, source, request)
            },
            SystemCallMessageHeader::ShutdownSocketRequest => {
                let request: ShutdownSocketRequest =
                    ShutdownSocketRequest::from_bytes(message.payload);
                sys_socket::do_shutdown(&syscall_table, source, request)
            },
            SystemCallMessageHeader::TimesRequest => {
                let request: TimesRequest = TimesRequest::from_bytes(message.payload);
                sys_times::do_times(&syscall_table, source, request)
            },
            SystemCallMessageHeader::UpdateFileAccessTimeRequest => {
                let request: UpdateFileAccessTimeRequest =
                    UpdateFileAccessTimeRequest::from_bytes(message.payload)?;
                fcntl::do_futimens(&syscall_table, source, request)
            },
            SystemCallMessageHeader::PipeRequest => {
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

    ///
    /// # Description
    ///
    /// Dispatches syscall requests whose request data spans multiple messages.
    ///
    /// # Parameters
    ///
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `assembler`: Shared request assembler for multi-part messages.
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `message`: The parsed linuxd daemon message.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_long_request_messages<T>(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        message: SystemCallMessage,
    ) -> Result<(), WorkerThreadError> {
        match message.header {
            SystemCallMessageHeader::ChangeDirectoryRequestPart => {
                Self::handle_long_request::<T, ChangeDirectoryRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::FileAccessAtRequestPart => {
                Self::handle_long_request::<T, FileAccessAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::FileStatAtRequestPart => {
                Self::handle_long_request::<T, FileStatAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::SymbolicLinkAtRequestPart => {
                Self::handle_long_request::<T, SymbolicLinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::LinkAtRequestPart => {
                Self::handle_long_request::<T, LinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::ReadLinkAtRequestPart => {
                Self::handle_long_request::<T, ReadLinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::MakeDirectoryAtRequestPart => {
                Self::handle_long_request::<T, MakeDirectoryAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::UpdateFileAccessTimeAtRequestPart => {
                Self::handle_long_request::<T, UpdateFileAccessTimeAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::FileChownAtRequestPart => {
                Self::handle_long_request::<T, FileChownAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::FileChmodAtRequestPart => {
                Self::handle_long_request::<T, FileChmodAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::OpenAtRequestPart => {
                Self::handle_long_request::<T, OpenAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::RenameAtRequestPart => {
                Self::handle_long_request::<T, RenameAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::UnlinkAtRequestPart => {
                Self::handle_long_request::<T, UnlinkAtRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::PollRequestPart => {
                Self::handle_long_request::<T, PollRequest>(
                    uvm_stream,
                    assembler,
                    syscall_table,
                    source,
                    &message,
                )
            },
            SystemCallMessageHeader::SelectRequestPart => {
                Self::handle_long_request::<T, SelectRequest>(
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

    ///
    /// # Description
    ///
    /// Dispatches syscall requests whose response data spans multiple messages.
    ///
    /// # Parameters
    ///
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `message`: The parsed linuxd daemon message.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_long_response_messages<T>(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        message: SystemCallMessage,
    ) -> Result<(), WorkerThreadError> {
        match message.header {
            SystemCallMessageHeader::FileStatRequest => {
                Self::handle_fstat_request(syscall_table, uvm_stream, source, message)
            },
            SystemCallMessageHeader::GetCurrentWorkingDirectoryRequest => {
                Self::handle_getcwd_request(uvm_stream, syscall_table, source)
            },
            SystemCallMessageHeader::GetDirectoryEntriesRequest => {
                Self::handle_getdents_request(syscall_table, uvm_stream, source, message)
            },
            header => {
                // The following statement is unreachable, because the matching logic in this
                // function should match the one in the `Self::run()` function.
                unreachable!("unexpected long response message {:?}", header)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Sends an IKC message frame to the user VM over the shared writer stream.
    ///
    /// # Parameters
    ///
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `message`: The IKC message to send.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an I/O error if the write fails.
    ///
    async fn send(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        message: Message,
    ) -> Result<(), std::io::Error> {
        // Coalesce frame type byte and message payload into a single write to reduce
        // syscall overhead and avoid sending the frame byte as a separate tiny segment.
        let msg_bytes: [u8; std::mem::size_of::<Message>()] = message.to_bytes();
        let mut buf: [u8; 1 + std::mem::size_of::<Message>()] =
            [0; 1 + std::mem::size_of::<Message>()];
        buf[0] = IkcFrame::MESSAGE_FRAME;
        buf[1..].copy_from_slice(&msg_bytes);
        let mut guard: MutexGuard<'_, SocketStreamWriter> = uvm_stream.lock().await;
        guard.write_all(&buf).await?;
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Sends a data chunk transfer to the user VM.  The frame is: frame type byte + 4-byte LE
    /// length prefix + serialized [`DataChunk`](::sys::ipc::DataChunk) payload (header + data).
    ///
    /// # Parameters
    ///
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `bulk`: The data chunk to send.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an I/O error if the write fails.
    ///
    async fn send_bulk(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        bulk: &::sys::ipc::DataChunk,
    ) -> Result<(), std::io::Error> {
        let payload: Vec<u8> = bulk.to_bytes();
        let payload_len: u32 = u32::try_from(payload.len()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "bulk payload length exceeds u32")
        })?;
        let len_prefix: [u8; 4] = payload_len.to_le_bytes();
        // Coalesce frame type byte, length prefix, and payload into a single vectored write
        // to reduce syscall overhead and avoid extra allocation+copy.
        let frame_byte: [u8; 1] = [IkcFrame::DATA_CHUNK_FRAME];
        let mut guard: MutexGuard<'_, SocketStreamWriter> = uvm_stream.lock().await;
        guard
            .write_all_vectored(&mut [
                std::io::IoSlice::new(&frame_byte),
                std::io::IoSlice::new(&len_prefix),
                std::io::IoSlice::new(&payload),
            ])
            .await?;
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Builds an error [`Message`] addressed to `source` with the given [`ErrorCode`].
    ///
    /// # Parameters
    ///
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `code`: The error code to embed in the response.
    ///
    /// # Returns
    ///
    /// An IKC error [`Message`].
    ///
    fn do_error(source: ThreadIdentifier, code: ErrorCode) -> Message {
        Message::new(
            MessageSender::new(LINUXD, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(source)), source),
            MessageType::Ikc,
            Some(code),
            [0u8; Message::PAYLOAD_SIZE],
        )
    }

    ///
    /// # Description
    ///
    /// Handles a close request.  Standard file descriptors (stdin, stdout, stderr) are faked
    /// because they are shared with the host process; all other fds are closed via the syscall
    /// table.
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `request`: The close request payload.
    ///
    /// # Returns
    ///
    /// The response [`Message`] on success, or a [`WorkerThreadError`] on failure.
    ///
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
                Ok(CloseResponse::build(source, 0, ::syscall::LINUXD, ::sys::ipc::MessageType::Ikc))
            },
            // Closing other file descriptors.
            _ => unistd::do_close(syscall_table, source, request),
        }
    }

    ///
    /// # Description
    ///
    /// Handles a write request.  Receives the bulk data payload from the channel, then either
    /// writes to the gateway stream (for stdout/stderr) as a cancellable async operation, or
    /// delegates to the syscall table for other file descriptors.
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `gateway_writer`: Shared writer stream for the gateway socket.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `request`: The write request payload.
    /// - `channel_rx`: Worker command channel receiver (for bulk data).
    /// - `cancel_rx`: Watch receiver used to detect cancellation.
    ///
    /// # Returns
    ///
    /// The response [`Message`] on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_write_request<T>(
        syscall_table: &SyscallTable<T>,
        gateway_writer: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        request: WriteRequest,
        channel_rx: &mut Receiver<VenvCommand>,
        cancel_rx: &mut watch::Receiver<bool>,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_write_request(): source={source:?}, request={request:?}");

        // Receive bulk data that carries the actual write payload. A timeout prevents the worker
        // thread from blocking forever if the guest VM crashes mid-protocol.
        let mut bulk_data: Vec<u8> =
            match Self::recv_with_timeout(channel_rx, BULK_DATA_TIMEOUT, cancel_rx) {
                Ok(VenvCommand::BulkData(bulk)) => bulk.into_data(),
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

        // Label: linuxd::worker_thread::handle_write_request()
        profiler::timestamp_message!(&mut bulk_data, 0);

        let count: usize = request.count as usize;
        let write_buf: &[u8] = if bulk_data.len() >= count {
            &bulk_data[..count]
        } else {
            &bulk_data
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
                let result: ::std::io::Result<()> = Self::run_cancellable_operation(
                    locked_gateway_writer.write_all(write_buf),
                    cancel_rx,
                );

                match result {
                    Ok(()) => {
                        debug!("wrote {} bytes to the gateway", write_buf.len());
                        Ok(WriteResponse::build(
                            source,
                            write_buf.len() as i32,
                            ::syscall::LINUXD,
                            ::sys::ipc::MessageType::Ikc,
                        ))
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
                Ok(WriteResponse::build(
                    source,
                    ret as i32,
                    ::syscall::LINUXD,
                    ::sys::ipc::MessageType::Ikc,
                ))
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

    ///
    /// # Description
    ///
    /// Handles a socket `send()` request.  Receives the bulk data that carries the payload from
    /// the channel, then forwards it to the networking backend.  The payload travels out-of-band
    /// via a page-bounded scatter/gather push, mirroring [`Self::handle_write_request`].
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `request`: The send request payload.
    /// - `channel_rx`: Worker command channel receiver (for bulk data).
    /// - `cancel_rx`: Watch receiver used to detect cancellation.
    ///
    /// # Returns
    ///
    /// The response [`Message`] on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_send_request<T>(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: SendSocketRequest,
        channel_rx: &mut Receiver<VenvCommand>,
        cancel_rx: &mut watch::Receiver<bool>,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_send_request(): source={source:?}, request={request:?}");

        // Receive bulk data that carries the actual send payload. A timeout prevents the worker
        // thread from blocking forever if the guest VM crashes mid-protocol.
        let bulk_data: Vec<u8> =
            match Self::recv_with_timeout(channel_rx, BULK_DATA_TIMEOUT, cancel_rx) {
                Ok(VenvCommand::BulkData(bulk)) => bulk.into_data(),
                Ok(VenvCommand::Shutdown) => {
                    debug!("handle_send_request(): received shutdown while waiting for bulk data");
                    return Err(WorkerThreadError::Interrupted);
                },
                Ok(VenvCommand::Work(_)) => {
                    error!("handle_send_request(): expected bulk data, got IKC message");
                    return Ok(build_error(source, ErrorCode::InvalidMessage));
                },
                Err(e) => {
                    error!("handle_send_request(): failed to receive bulk data");
                    return Err(e);
                },
            };

        sys_socket::do_send(syscall_table, source, request, &bulk_data)
    }

    ///
    /// # Description
    ///
    /// Handles a read request.  Receives the bulk data pull header from the channel, then
    /// either reads from the gateway stream (for stdin) as a cancellable async operation, or
    /// delegates to the syscall table for other file descriptors.  The read data is sent back
    /// to the kernel via a bulk response.
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `gateway_reader`: Shared reader stream for the gateway socket.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `request`: The read request payload.
    /// - `channel_rx`: Worker command channel receiver (for bulk data).
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `cancel_rx`: Watch receiver used to detect cancellation.
    ///
    /// # Returns
    ///
    /// The response [`Message`] on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_read_request<T>(
        syscall_table: &SyscallTable<T>,
        gateway_reader: Arc<Mutex<SocketStreamReader>>,
        source: ThreadIdentifier,
        request: ReadRequest,
        channel_rx: &mut Receiver<VenvCommand>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        cancel_rx: &mut watch::Receiver<bool>,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_read_request(): source={source:?}, request={request:?}");

        // Wait for the BulkData pull request from the kernel. This contains the kernel buffer
        // address where the response data should be written. A timeout prevents the worker thread
        // from blocking forever if the guest VM crashes mid-protocol.
        let pull_header: ::sys::ipc::DataChunkHeader =
            match Self::recv_with_timeout(channel_rx, BULK_DATA_TIMEOUT, cancel_rx) {
                Ok(VenvCommand::BulkData(bulk)) => *bulk.header(),
                Ok(VenvCommand::Shutdown) => {
                    debug!("handle_read_request(): received shutdown while waiting for bulk data");
                    return Err(WorkerThreadError::Interrupted);
                },
                Ok(VenvCommand::Work(_)) => {
                    error!("handle_read_request(): expected BulkData, got IKC message");
                    return Ok(build_error(source, ErrorCode::InvalidMessage));
                },
                Err(e) => {
                    error!("handle_read_request(): failed to receive bulk data");
                    return Err(e);
                },
            };

        let return_addr: u32 = pull_header.data_addr();
        let max_len: usize = pull_header.data_len() as usize;

        // Helper closure: send a bulk response back to the kernel buffer.
        let send_bulk_response = |data: Vec<u8>, len: u32| -> Result<(), WorkerThreadError> {
            let bulk: ::sys::ipc::DataChunk = ::sys::ipc::DataChunk::new(
                ::sys::ipc::DataChunkHeader::new(
                    pull_header.source_pid(),
                    pull_header.source_tid(),
                    pull_header.destination_pid(),
                    pull_header.destination_tid(),
                    return_addr,
                    len,
                ),
                data,
            );
            Handle::current()
                .block_on(Self::send_bulk(uvm_stream.clone(), &bulk))
                .map_err(|e| {
                    if e.kind() == ErrorKind::BrokenPipe {
                        debug!("handle_read_request(): UVM stream closed (broken pipe)");
                        WorkerThreadError::Interrupted
                    } else {
                        error!("handle_read_request(): failed to send bulk response (error={e:?})");
                        WorkerThreadError::Interrupted
                    }
                })
        };

        if request.fd == STDIN_FILENO {
            // Read from gateway.
            let mut locked_gateway_reader: MutexGuard<'_, SocketStreamReader> =
                gateway_reader.blocking_lock();

            let mut read_buf: Vec<u8> = ::std::vec![0u8; max_len];
            let result: ::std::io::Result<usize> = Self::run_cancellable_operation(
                locked_gateway_reader.read(&mut read_buf),
                cancel_rx,
            );
            drop(locked_gateway_reader);

            match result {
                Ok(0) => {
                    debug!("handle_read_request(): eof");
                    send_bulk_response(Vec::new(), 0)?;
                    Ok(ReadResponse::eof(source, ::syscall::LINUXD, ::sys::ipc::MessageType::Ikc))
                },
                Ok(n) => {
                    debug!("read {n} bytes from gateway");
                    read_buf.truncate(n);
                    // Label: linuxd::worker_thread::handle_read_request()
                    profiler::timestamp_message!(&mut read_buf, 0);
                    send_bulk_response(read_buf, n as u32)?;
                    let empty_buf: [u8; ReadResponse::BUFFER_SIZE] =
                        [0u8; ReadResponse::BUFFER_SIZE];
                    Ok(ReadResponse::build(
                        source,
                        n as c_ssize_t,
                        empty_buf,
                        ::syscall::LINUXD,
                        ::sys::ipc::MessageType::Ikc,
                    ))
                },
                Err(e) if e.kind() == ErrorKind::Interrupted => {
                    debug!("handle_read_request(): read interrupted");
                    // Send empty bulk to unblock the kernel pull before returning.
                    if let Err(bulk_err) = send_bulk_response(Vec::new(), 0) {
                        warn!(
                            "handle_read_request(): failed to send empty bulk response on \
                             interrupt, kernel pull thread may block (error={bulk_err:?})"
                        );
                    }
                    Err(WorkerThreadError::Interrupted)
                },
                Err(e) => {
                    error!("handle_read_request(): error reading data from gateway (error={e:?})");
                    send_bulk_response(Vec::new(), 0)?;
                    Ok(ReadResponse::eof(source, ::syscall::LINUXD, ::sys::ipc::MessageType::Ikc))
                },
            }
        } else {
            // Read from other file descriptor via syscall table redirection with bulk support.
            let fd: libc::c_int = request.fd;
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
                send_bulk_response(read_buf, n as u32)?;
                let empty_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
                Ok(ReadResponse::build(
                    source,
                    n as c_ssize_t,
                    empty_buf,
                    ::syscall::LINUXD,
                    ::sys::ipc::MessageType::Ikc,
                ))
            } else if ret == 0 {
                send_bulk_response(Vec::new(), 0)?;
                Ok(ReadResponse::eof(source, ::syscall::LINUXD, ::sys::ipc::MessageType::Ikc))
            } else {
                let errno: i32 = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    error!(
                        "handle_read_request(): worker thread interrupted while blocked on read()"
                    );
                    if let Err(bulk_err) = send_bulk_response(Vec::new(), 0) {
                        warn!(
                            "handle_read_request(): failed to send empty bulk response on \
                             interrupt, kernel pull thread may block (error={bulk_err:?})"
                        );
                    }
                    Err(WorkerThreadError::Interrupted)
                } else {
                    error!("handle_read_request(): read via syscall table failed (errno={errno})");
                    send_bulk_response(Vec::new(), 0)?;
                    Ok(build_error(
                        source,
                        ErrorCode::try_from(errno).unwrap_or_else(|_| {
                            error!(
                                "handle_read_request(): unmapped errno={errno}, falling back to \
                                 IoErr"
                            );
                            ErrorCode::IoErr
                        }),
                    ))
                }
            }
        }
    }

    ///
    /// # Description
    ///
    /// Handles a `recv()` request.  Waits for the pull-header bulk frame the guest's `ipc::pull()`
    /// emits after the request, performs the receive on the networking backend, pushes the
    /// received payload directly into the guest buffer, and returns the response message.
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `request`: The receive request payload.
    /// - `channel_rx`: Worker command channel receiver (for bulk data).
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `cancel_rx`: Watch receiver used to detect cancellation.
    ///
    /// # Returns
    ///
    /// The response [`Message`] on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_recv_request<T>(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: ReceiveSocketRequest,
        channel_rx: &mut Receiver<VenvCommand>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        cancel_rx: &mut watch::Receiver<bool>,
    ) -> Result<Message, WorkerThreadError> {
        trace!("handle_recv_request(): source={source:?}, request={request:?}");

        // Wait for the BulkData pull request that the guest's `ipc::pull()` emits after the
        // request. It carries the kernel buffer address where the payload must be written. A
        // timeout prevents the worker thread from blocking forever if the guest VM crashes
        // mid-protocol.
        let pull_header: ::sys::ipc::DataChunkHeader =
            match Self::recv_with_timeout(channel_rx, BULK_DATA_TIMEOUT, cancel_rx) {
                Ok(VenvCommand::BulkData(bulk)) => *bulk.header(),
                Ok(VenvCommand::Shutdown) => {
                    debug!("handle_recv_request(): received shutdown while waiting for bulk data");
                    return Err(WorkerThreadError::Interrupted);
                },
                Ok(VenvCommand::Work(_)) => {
                    error!("handle_recv_request(): expected BulkData, got IKC message");
                    return Ok(build_error(source, ErrorCode::InvalidMessage));
                },
                Err(e) => {
                    error!("handle_recv_request(): failed to receive bulk data");
                    return Err(e);
                },
            };

        // Helper closure: push a bulk payload back into the guest buffer.
        let send_bulk_response = |data: Vec<u8>, len: u32| -> Result<(), WorkerThreadError> {
            let bulk: ::sys::ipc::DataChunk = ::sys::ipc::DataChunk::new(
                ::sys::ipc::DataChunkHeader::new(
                    pull_header.source_pid(),
                    pull_header.source_tid(),
                    pull_header.destination_pid(),
                    pull_header.destination_tid(),
                    pull_header.data_addr(),
                    len,
                ),
                data,
            );
            Handle::current()
                .block_on(Self::send_bulk(uvm_stream.clone(), &bulk))
                .map_err(|e| {
                    if e.kind() == ErrorKind::BrokenPipe {
                        debug!("handle_recv_request(): UVM stream closed (broken pipe)");
                    } else {
                        error!("handle_recv_request(): failed to send bulk response (error={e:?})");
                    }
                    WorkerThreadError::Interrupted
                })
        };

        // Perform the receive on the networking backend.
        match sys_socket::do_recv(syscall_table, source, request) {
            Ok((response, data)) => {
                // The payload is bounded by the scatter/gather bulk limit, so its length fits in
                // the bulk header field.
                let len: u32 = data.len() as u32;
                send_bulk_response(data, len)?;
                Ok(response)
            },
            Err(WorkerThreadError::Interrupted) => {
                // Release the guest blocked in `ipc::pull()` with an empty transfer before
                // returning so the kernel pull thread does not block.
                if let Err(bulk_err) = send_bulk_response(Vec::new(), 0) {
                    warn!(
                        "handle_recv_request(): failed to send empty bulk response on interrupt, \
                         kernel pull thread may block (error={bulk_err:?})"
                    );
                }
                Err(WorkerThreadError::Interrupted)
            },
            Err(e) => Err(e),
        }
    }

    ///
    /// # Description
    ///
    /// Handles a file stat request and sends the multi-message response to the user VM.
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `message`: The parsed linuxd daemon message.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_fstat_request<T>(
        syscall_table: &SyscallTable<T>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        message: SystemCallMessage,
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

    ///
    /// # Description
    ///
    /// Handles a get-current-directory request and sends the multi-message response.
    ///
    /// # Parameters
    ///
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `source`: Thread identifier of the requesting guest thread.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a [`WorkerThreadError`] on failure.
    ///
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

    ///
    /// # Description
    ///
    /// Handles a get-directory-entries request and sends the multi-message response.
    ///
    /// # Parameters
    ///
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `message`: The parsed linuxd daemon message.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_getdents_request<T>(
        syscall_table: &SyscallTable<T>,
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        source: ThreadIdentifier,
        message: SystemCallMessage,
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

    ///
    /// # Description
    ///
    /// Assembles a multi-part request from its constituent message parts, then processes the
    /// assembled request and sends the response.  The assembler lock is held only during the
    /// assembly phase to avoid starving other worker threads.
    ///
    /// # Parameters
    ///
    /// - `uvm_stream`: Writer stream to the user VM.
    /// - `assembler`: Shared request assembler for multi-part messages.
    /// - `syscall_table`: Shared syscall dispatch table.
    /// - `source`: Thread identifier of the requesting guest thread.
    /// - `message`: The parsed linuxd daemon message.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a [`WorkerThreadError`] on failure.
    ///
    fn handle_long_request<S, T>(
        uvm_stream: Arc<Mutex<SocketStreamWriter>>,
        assembler: Arc<Mutex<RequestAssembler>>,
        syscall_table: &SyscallTable<S>,
        source: ThreadIdentifier,
        message: &SystemCallMessage,
    ) -> Result<(), WorkerThreadError>
    where
        T: RequestAssemblerTrait<S>,
    {
        let part: SystemCallMessagePart = SystemCallMessagePart::from_bytes(message.payload);

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
/// No-op signal handler for `INTERRUPT_SIGNAL`.  Its only purpose is to make blocking libc
/// syscalls return `EINTR` so the worker thread can check for cancellation.
///
/// This function is async-signal-safe: it performs no heap allocations, locking, or I/O.
///
extern "C" fn linuxd_worker_thread_signal_handler(_: c_int) {}
