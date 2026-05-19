// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone User VM handle.
//!
//! This module encapsulates the logic for spawning and awaiting a User VM in standalone mode
//! (no system VM, control-plane, or gateway). It creates isolated channels, starts the VM, and
//! runs an I/O handler task that processes guest IKC messages (WriteRequest/ReadRequest) so that
//! guest stdout/stdin can be bridged to external consumers such as nanvix-terminal or
//! nanvix-http.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    CHANNEL_CAPACITY,
    UserVm,
    UserVmArgs,
    counters::MessageCounters,
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
    },
};
use ::anyhow::Result;
use ::hostfsd::HostFsHandler;
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::nanvix_sandbox_config::NetworkingMode;
use ::networkd::NetworkDaemon;
use ::std::{
    collections::VecDeque,
    path::PathBuf,
    sync::Arc,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        DataChunk,
        DataChunkHeader,
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
use ::syscall::{
    SystemCallMessage,
    SystemCallMessageHeader,
    unistd::message::{
        ReadRequest,
        ReadResponse,
        WriteRequest,
        WriteResponse,
    },
};
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
};

#[cfg(feature = "profile-time")]
use crate::perf::PerfTimings;

//==================================================================================================
// Type Aliases
//==================================================================================================

/// Payload sent to the hostfsd worker thread: the IKC message, a channel for the
/// response, and the shared message counters.
type HostFsRequest = (Message, mpsc::Sender<IkcFrame>, MessageCounters);

//==================================================================================================
// Constants
//==================================================================================================

/// Standard POSIX file descriptors for FD validation in I/O handlers.
const STDIN_FILENO: i32 = 0;
const STDOUT_FILENO: i32 = 1;
const STDERR_FILENO: i32 = 2;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// I/O channels for communicating with a standalone User VM.
///
/// Provides the external interface for bridging host I/O with the guest's stdin/stdout via IKC.
/// Consumers (e.g., nanvix-terminal, nanvix-http) use these channels to send input data to the
/// guest and receive output data from the guest.
///
/// If this struct is dropped without being used, the I/O handler falls back to discarding output
/// and signaling EOF on input — preserving backward-compatible standalone behavior.
///
pub struct StandaloneVmIo {
    /// Receives application data written by the guest to stdout/stderr.
    pub output_rx: mpsc::Receiver<Vec<u8>>,
    /// Sends application data to be read by the guest from stdin.
    pub input_tx: mpsc::Sender<Vec<u8>>,
}

///
/// # Description
///
/// Handle to a User VM running in standalone mode.
///
/// Bundles the VMM task handle and the I/O handler task so that callers can await or abort the
/// VM as a single unit. Created via [`StandaloneVmHandle::spawn`].
///
pub struct StandaloneVmHandle {
    /// Task running the VM.
    vmm_handle: JoinHandle<Result<u16>>,
    /// Task processing IKC messages between the guest and external I/O channels.
    io_handle: JoinHandle<()>,
    /// Kept alive so the orchestrator's `io_control_rx` does not see an immediate channel close,
    /// which would cause it to exit the run loop before the guest starts executing.
    _io_cmd_tx: mpsc::Sender<IoControlCommand>,
    /// Kept alive so the orchestrator can send control responses without a closed-channel error.
    _io_resp_rx: mpsc::Receiver<IoControlResponse>,
    /// Performance timings collector for fine-grained startup breakdown.
    #[cfg(feature = "profile-time")]
    perf_timings: PerfTimings,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl StandaloneVmHandle {
    ///
    /// # Description
    ///
    /// Spawns a User VM in standalone mode with isolated channels.
    ///
    /// Creates the channel plumbing required by [`UserVm::spawn`], starts the VMM, and spawns an
    /// I/O handler task that processes guest IKC messages (WriteRequest/ReadRequest) and bridges
    /// them to external I/O channels exposed via [`StandaloneVmIo`].
    ///
    /// # Parameters
    ///
    /// - `kernel_filename`: Path to the kernel binary.
    /// - `initrd_filename`: Optional path to the initrd payload.
    /// - `initrd_args`: Optional arguments forwarded to the initrd payload.
    /// - `kernel_args`: Optional kernel arguments written to guest control registers.
    /// - `ramfs_filename`: Optional path to a RAM filesystem image.
    /// - `stderr`: Optional path to a file used to capture the guest's stderr stream.
    /// - `snapshot_path`: Optional path to a snapshot from which to restore VM state instead of
    ///   cold-booting.
    ///
    /// # Returns
    ///
    /// A tuple containing the VM handle (for awaiting/aborting) and the I/O channels (for
    /// bridging host I/O with the guest).
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        kernel_filename: String,
        initrd_filename: Option<String>,
        initrd_args: Option<String>,
        kernel_args: Option<String>,
        ramfs_filename: Option<String>,
        stderr: Option<String>,
        snapshot_path: Option<String>,
        mount_directory: Option<String>,
        networking_mode: NetworkingMode,
        #[cfg(feature = "gdb")] gdb_port: Option<u16>,
    ) -> (Self, StandaloneVmIo) {
        // Create internal VM channels. In standalone mode these are wired directly without an
        // I/O thread.
        let (vcpu_thread_stdout_tx, standalone_data_rx) =
            mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
        let (inbound_data_tx, memory_thread_data_rx) = mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
        // Kept alive so the orchestrator's io_control_rx does not see an immediate channel close.
        let (io_cmd_tx, io_control_rx) = mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
        // Kept alive so the orchestrator can send control responses without a closed-channel
        // error.
        let (io_control_tx, io_resp_rx) = mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);

        // Create external I/O channels for consumers (terminal, HTTP gateway, etc.).
        let (output_tx, output_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_CAPACITY);
        let (input_tx, input_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_CAPACITY);

        let counters: MessageCounters = MessageCounters::new();
        let io_counters: MessageCounters = counters.clone();

        #[cfg(feature = "profile-time")]
        let perf_timings: PerfTimings = PerfTimings::new();

        let vmm_handle: JoinHandle<Result<u16>> = UserVm::spawn(UserVmArgs {
            initrd_filename,
            initrd_args,
            kernel_args,
            ramfs_filename,
            stderr,
            vcpu_thread_stdout_tx,
            memory_thread_data_rx,
            io_control_rx,
            io_control_tx,
            kernel_filename,
            counters,
            snapshot_path,
            #[cfg(feature = "gdb")]
            gdb_port,
            #[cfg(feature = "profile-time")]
            perf_timings: perf_timings.clone(),
            guest_profile_path: std::env::var("NANVIX_GUEST_PROFILE_PATH").ok(),
        });

        // Spawn the I/O handler task that processes guest IKC messages and bridges them to the
        // external I/O channels.
        let io_handle: JoinHandle<()> = tokio::spawn(async move {
            standalone_io_handler(
                standalone_data_rx,
                inbound_data_tx,
                output_tx,
                input_rx,
                io_counters,
                networking_mode,
                mount_directory,
            )
            .await;
        });

        let handle: Self = Self {
            vmm_handle,
            io_handle,
            _io_cmd_tx: io_cmd_tx,
            _io_resp_rx: io_resp_rx,
            #[cfg(feature = "profile-time")]
            perf_timings,
        };

        let io: StandaloneVmIo = StandaloneVmIo {
            output_rx,
            input_tx,
        };

        (handle, io)
    }

    ///
    /// # Description
    ///
    /// Waits for the VM to finish and returns its exit status.
    ///
    /// Awaits both the VMM task and the I/O handler task, then returns the raw exit status
    /// reported by the guest. The I/O handler task is always awaited regardless of the VM's
    /// outcome.
    ///
    /// # Returns
    ///
    /// On success, returns the guest's exit status as a `u16`. On failure, returns an error if
    /// the VM task panicked or the VM itself failed.
    ///
    pub async fn wait(self) -> Result<u16> {
        let vm_exit_status: Result<u16> = self.vmm_handle.await?;
        debug!("standalone: VM completed (exit_status={vm_exit_status:?})");

        // Wait for the I/O handler to finish.
        if let Err(error) = self.io_handle.await {
            warn!("standalone: I/O handler task failed (error={error:?})");
        }

        // Emit performance timings to host stderr so the benchmark can parse them.
        #[cfg(feature = "profile-time")]
        self.perf_timings.emit_to_stderr();

        vm_exit_status
    }

    ///
    /// # Description
    ///
    /// Aborts the VM and I/O handler tasks without waiting for completion.
    ///
    pub fn abort(&self) {
        self.vmm_handle.abort();
        self.io_handle.abort();
    }

    ///
    /// # Description
    ///
    /// Aborts then awaits both the VM and I/O handler tasks to ensure clean shutdown.
    ///
    pub async fn abort_and_wait(self) {
        self.vmm_handle.abort();
        self.io_handle.abort();
        let _ = self.vmm_handle.await;
        let _ = self.io_handle.await;
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Extracts the [`ThreadIdentifier`] from a message sender field.
///
/// Write/Read requests encode the originating thread as a negative value in the
/// [`MessageSender`](::sys::ipc::MessageSender) field.
///
fn extract_tid(source: ::sys::ipc::MessageSender) -> ThreadIdentifier {
    match source.as_id() {
        Err(tid) => tid,
        Ok(_pid) => {
            warn!("standalone io_handler: message source is a PID, expected TID");
            ThreadIdentifier::from(1i32)
        },
    }
}

///
/// # Description
///
/// Processes guest IKC messages in standalone mode, bridging the VM's IKC channel to external
/// consumer channels.
///
/// This handler replaces linuxd for standalone deployments: it receives WriteRequest and
/// ReadRequest messages from the guest via the VM's stdout channel, processes them, and sends
/// responses back via the VM's stdin channel. Application data is forwarded to/from external
/// channels that consumers (terminal, HTTP gateway) use.
///
/// # Parameters
///
/// - `vm_stdout_rx`: Receives IKC frames emitted by the guest (via `output_fn`).
/// - `vm_stdin_tx`: Sends IKC frames to the guest (consumed by `input_fn`).
/// - `output_tx`: Forwards application data written by the guest to the external consumer.
/// - `input_rx`: Receives application data from the external consumer for guest reads.
///
async fn standalone_io_handler(
    mut vm_stdout_rx: mpsc::Receiver<IkcFrame>,
    vm_stdin_tx: mpsc::Sender<IkcFrame>,
    output_tx: mpsc::Sender<Vec<u8>>,
    mut input_rx: mpsc::Receiver<Vec<u8>>,
    counters: MessageCounters,
    networking_mode: NetworkingMode,
    mount_directory: Option<String>,
) {
    let mut input_buffer: VecDeque<u8> = VecDeque::new();
    let mut input_closed: bool = false;

    let network_daemon: Option<Arc<NetworkDaemon>> = if networking_mode.is_enabled() {
        match NetworkDaemon::new() {
            Ok(nd) => Some(Arc::new(nd)),
            Err(e) => {
                error!("standalone io_handler: failed to initialize network daemon: {e}");
                None
            },
        }
    } else {
        None
    };

    // Initialize the host filesystem daemon handler when a mount directory is specified.
    // A dedicated worker thread serializes request processing to avoid concurrent host
    // filesystem mutation and because the handler state (`HostFsHandler`) is not `Send`/`Sync`.
    //
    // The mount directory is validated eagerly (before spawning the worker) so that
    // invalid paths surface an error at VM startup rather than silently queuing
    // requests into a channel that drains into error responses.
    //
    // NOTE: `_hostfs_worker_handle` is currently unused. The worker thread terminates
    // when the channel sender (`hostfs_tx`) is dropped at the end of this function, which
    // causes `rx.recv()` to return `Err` and the loop to exit. Explicit join is not
    // performed because this function is async and `JoinHandle::join` is blocking.
    // TODO(#hostfs-shutdown): consider joining via `tokio::task::spawn_blocking` on
    // graceful shutdown or converting the worker to a tokio task.
    let (hostfs_tx, _hostfs_worker_handle): (
        Option<std::sync::mpsc::SyncSender<HostFsRequest>>,
        Option<std::thread::JoinHandle<()>>,
    ) = match mount_directory.as_ref() {
        Some(dir) => {
            let path: PathBuf = PathBuf::from(dir);
            // Validate the mount directory before spawning the worker thread.
            if !path.is_dir() {
                error!(
                    "standalone io_handler: mount directory does not exist or is not a directory: \
                     {dir:?}"
                );
                (None, None)
            } else {
                debug!("standalone io_handler: initializing hostfsd (root={dir:?})");
                let (tx, rx) = std::sync::mpsc::sync_channel::<HostFsRequest>(64);
                // Use a oneshot channel to confirm handler initialization before accepting
                // requests. This prevents messages from piling up in the channel if the
                // HostFsHandler fails to initialize inside the spawned thread.
                let (init_tx, init_rx) = std::sync::mpsc::sync_channel::<bool>(1);
                match std::thread::Builder::new()
                    .name("hostfsd-worker".into())
                    .spawn(move || {
                        let mut handler = match HostFsHandler::new(path) {
                            Ok(h) => {
                                let _ = init_tx.send(true);
                                h
                            },
                            Err(e) => {
                                error!("hostfsd-worker: failed to initialize handler: {e}");
                                let _ = init_tx.send(false);
                                return;
                            },
                        };
                        while let Ok((msg, response_tx, counters)) = rx.recv() {
                            let response_payload = handler.handle_request(&msg.payload);
                            let response: Message = Message::new(
                                MessageSender::from(ProcessIdentifier::KERNEL),
                                MessageReceiver::from(ProcessIdentifier::VFSD),
                                MessageType::Ikc,
                                None,
                                response_payload,
                            );
                            // NOTE: `increment_io_thread_messages_received()` counts
                            // messages flowing from the IO thread back into the VM,
                            // not messages the IO thread receives. The name is a
                            // project-wide convention; renaming is out of scope here.
                            counters.increment_io_thread_messages_received();
                            if response_tx
                                .blocking_send(IkcFrame::Message(response))
                                .is_err()
                            {
                                error!(
                                    "hostfsd-worker: failed to send response (VM input channel \
                                     closed)"
                                );
                                break;
                            }
                        }
                        debug!("hostfsd-worker: exiting");
                    }) {
                    Ok(handle) => {
                        // Wait for the worker to confirm handler initialization.
                        match init_rx.recv() {
                            Ok(true) => (Some(tx), Some(handle)),
                            _ => {
                                error!(
                                    "standalone io_handler: hostfsd worker failed to initialize"
                                );
                                (None, Some(handle))
                            },
                        }
                    },
                    Err(e) => {
                        error!("standalone io_handler: failed to spawn hostfsd worker thread: {e}");
                        (None, None)
                    },
                }
            }
        },
        None => (None, None),
    };

    trace!("standalone io_handler: entering receive loop");
    while let Some(frame) = vm_stdout_rx.recv().await {
        trace!("standalone io_handler: received frame (type={})", frame.frame_type_byte());
        match frame {
            IkcFrame::Message(msg) => {
                let syscall_msg: SystemCallMessage =
                    match SystemCallMessage::try_from_bytes(msg.payload) {
                        Ok(syscall_msg) => syscall_msg,
                        Err(e) => {
                            warn!("standalone io_handler: failed to parse message: {e:?}");
                            continue;
                        },
                    };

                let header = syscall_msg.header;
                match header {
                    SystemCallMessageHeader::WriteRequest => {
                        let tid: ThreadIdentifier = extract_tid(msg.source);
                        let req: WriteRequest = WriteRequest::from_bytes(syscall_msg.payload);
                        handle_write_request(
                            &mut vm_stdout_rx,
                            &vm_stdin_tx,
                            &output_tx,
                            tid,
                            &req,
                            &counters,
                        )
                        .await;
                    },
                    SystemCallMessageHeader::ReadRequest => {
                        let tid: ThreadIdentifier = extract_tid(msg.source);
                        let req: ReadRequest = ReadRequest::from_bytes(syscall_msg.payload);
                        handle_read_request(
                            &mut vm_stdout_rx,
                            &vm_stdin_tx,
                            &mut input_rx,
                            &mut input_buffer,
                            &mut input_closed,
                            tid,
                            &req,
                            &counters,
                        )
                        .await;
                    },
                    header if header.is_hostfs() => {
                        if let Some(ref tx) = hostfs_tx {
                            // Capture the op_id from the request payload BEFORE try_send
                            // consumes the message, so the error path can echo it back.
                            let request_op_id: ::hostfs_api::OperationId =
                                ::hostfs_api::get_op_id(&msg.payload);
                            if tx
                                .try_send((msg, vm_stdin_tx.clone(), counters.clone()))
                                .is_err()
                            {
                                error!(
                                    "standalone io_handler: hostfs worker channel full or closed"
                                );
                                send_hostfs_error(header, request_op_id, &vm_stdin_tx, &counters)
                                    .await;
                            }
                            continue;
                        }
                        // No mount directory configured: send an error response so
                        // vfsd can drain its pending queue and report the error to
                        // the caller instead of leaving them blocked.
                        warn!(
                            "standalone io_handler: hostfs message received but no mount \
                             configured; sending error response"
                        );
                        let request_op_id: ::hostfs_api::OperationId =
                            ::hostfs_api::get_op_id(&msg.payload);
                        send_hostfs_error(header, request_op_id, &vm_stdin_tx, &counters).await;
                    },
                    header => {
                        let destination = { msg.destination };
                        if destination == MessageReceiver::NETWORKD {
                            if let Some(ref nd) = network_daemon {
                                spawn_networking_task(
                                    nd.clone(),
                                    vm_stdin_tx.clone(),
                                    msg,
                                    counters.clone(),
                                );
                                continue;
                            }
                            // Networking not allowed: send an error response.
                            warn!(
                                "standalone io_handler: networking not allowed, rejecting message \
                                 with header {:?}",
                                header
                            );
                            let tid: ThreadIdentifier = extract_tid(msg.source);
                            let error_response: Message = Message::new(
                                MessageSender::NETWORKD,
                                MessageReceiver::from(tid),
                                MessageType::Ikc,
                                Some(ErrorCode::OperationNotSupported),
                                [0u8; Message::PAYLOAD_SIZE],
                            );
                            if vm_stdin_tx
                                .send(IkcFrame::Message(error_response))
                                .await
                                .is_err()
                            {
                                error!(
                                    "standalone io_handler: failed to send error response (VM \
                                     input channel closed)"
                                );
                            }
                            continue;
                        }
                        trace!("standalone io_handler: ignoring message with header {:?}", header);
                    },
                }
            },
            IkcFrame::Bulk(_) => {
                trace!("standalone io_handler: ignoring unexpected bulk frame");
            },
        }
    }

    debug!("standalone: I/O handler exiting (VM stdout channel closed)");
}

///
/// # Description
///
/// Sends an IKC error response for a hostfs request header back to vfsd.
///
/// Constructs a response message with a negative error indicator and sends it over `vm_stdin_tx`.
/// Used when the hostfs worker channel is full/closed or when no mount directory is configured.
///
/// Builds a per-operation error payload so that each vfsd completion handler detects the
/// failure correctly. Most operations check a leading i32 field for negative values; lseek
/// checks an i64 offset; stat uses all-zeros as its error sentinel; readdir uses name_len==0
/// as end-of-directory. Using 0xFF indiscriminately would produce bogus metadata for stat
/// (whose `size` field is `u64`).
///
/// The `op_id` from the original request is echoed into the response so that vfsd's
/// `PendingQueue::remove` can match it to the correct pending operation.
///
async fn send_hostfs_error(
    header: SystemCallMessageHeader,
    op_id: ::hostfs_api::OperationId,
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    counters: &MessageCounters,
) {
    let resp_header: SystemCallMessageHeader = match header.hostfs_response_header() {
        Some(resp) => resp,
        None => {
            error!("standalone io_handler: no response header for {header:?}");
            return;
        },
    };
    // Zero-initialize and write the appropriate error indicator per response type.
    let mut err_payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
    err_payload[0..2].copy_from_slice(&(resp_header as u16).to_ne_bytes());
    // Echo the original request's op_id so vfsd can match this error to the pending op.
    ::hostfs_api::set_op_id(&mut err_payload, op_id);

    let ds: usize = ::hostfs_api::HOSTFS_DATA_START;
    match resp_header {
        SystemCallMessageHeader::HostFsLseekResponse => {
            // Lseek completion checks offset as i64 < 0.
            err_payload[ds..ds + 8]
                .copy_from_slice(&(::hostfs_api::HOSTFS_ERR_IO as i64).to_le_bytes());
        },
        SystemCallMessageHeader::HostFsStatResponse
        | SystemCallMessageHeader::HostFsReadDirResponse => {
            // Stat uses all-zeros (size==0 && mode==0 && is_dir==0) as error sentinel.
            // Readdir uses name_len==0 as end-of-directory signal.
            // Zeros are already in place from the initialization above.
        },
        _ => {
            // All other operations check a leading i32 for negative values.
            err_payload[ds..ds + 4].copy_from_slice(&::hostfs_api::HOSTFS_ERR_IO.to_le_bytes());
        },
    }

    let err_response: Message = Message::new(
        MessageSender::from(ProcessIdentifier::KERNEL),
        MessageReceiver::from(ProcessIdentifier::VFSD),
        MessageType::Ikc,
        None,
        err_payload,
    );
    counters.increment_io_thread_messages_received();
    if vm_stdin_tx
        .send(IkcFrame::Message(err_response))
        .await
        .is_err()
    {
        error!("standalone io_handler: failed to send hostfs error response");
    }
}

///
/// # Description
///
/// Spawns a blocking task to handle a networking system call message.
///
/// Each networking operation runs on its own thread from tokio's blocking thread pool, preventing
/// blocking libc calls (accept, recv, connect, etc.) from stalling the main IO handler loop.
///
fn spawn_networking_task(
    network_daemon: Arc<NetworkDaemon>,
    vm_stdin_tx: mpsc::Sender<IkcFrame>,
    msg: Message,
    counters: MessageCounters,
) {
    trace!("standalone io_handler: spawning networking task");

    let handle = tokio::task::spawn_blocking(move || match network_daemon.handle_message(msg) {
        Some(responses) => {
            for response in responses {
                counters.increment_io_thread_messages_received();
                if vm_stdin_tx
                    .blocking_send(IkcFrame::Message(response))
                    .is_err()
                {
                    error!(
                        "standalone io_handler: failed to send networking response (VM input \
                         channel closed)"
                    );
                    return;
                }
            }
        },
        None => {
            warn!("standalone io_handler: networkd failed to handle message");
        },
    });

    tokio::spawn(async move {
        if let Err(e) = handle.await {
            error!("standalone io_handler: networking task panicked: {e}");
        }
    });
}

///
/// # Description
///
/// Handles a guest WriteRequest by consuming the subsequent bulk data frame, forwarding the
/// application data to the external output channel, and sending a WriteResponse back to the
/// guest.
///
async fn handle_write_request(
    vm_stdout_rx: &mut mpsc::Receiver<IkcFrame>,
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    output_tx: &mpsc::Sender<Vec<u8>>,
    tid: ThreadIdentifier,
    request: &WriteRequest,
    counters: &MessageCounters,
) {
    let fd: i32 = request.fd;
    trace!("standalone io_handler: handling WriteRequest (fd={fd}, tid={tid:?})");

    // Wait for the bulk data frame that follows the WriteRequest.
    let data: Vec<u8> = match vm_stdout_rx.recv().await {
        Some(IkcFrame::Bulk(bulk)) => bulk.into_data(),
        other => {
            error!(
                "standalone io_handler: expected bulk frame after WriteRequest, got {:?}",
                other.as_ref().map(|f| f.frame_type_byte())
            );
            let response: Message =
                WriteResponse::build(tid, 0, ProcessIdentifier::KERNEL, MessageType::Ikc);
            counters.increment_io_thread_messages_received();
            if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
                error!(
                    "standalone io_handler: failed to send WriteResponse (VM input channel closed)"
                );
            }
            return;
        },
    };

    // Only bridge writes to stdout/stderr; reject other FDs.
    if fd != STDOUT_FILENO && fd != STDERR_FILENO {
        warn!("standalone io_handler: rejecting write to unsupported fd={fd} (tid={tid:?})");
        let response: Message =
            WriteResponse::build(tid, -1, ProcessIdentifier::KERNEL, MessageType::Ikc);
        counters.increment_io_thread_messages_received();
        if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
            error!("standalone io_handler: failed to send WriteResponse (VM input channel closed)");
        }
        return;
    }

    let written: i32 = match i32::try_from(data.len()) {
        Ok(n) => n,
        Err(_) => {
            error!("standalone io_handler: write size overflows i32 (len={})", data.len());
            let response: Message =
                WriteResponse::build(tid, -1, ProcessIdentifier::KERNEL, MessageType::Ikc);
            counters.increment_io_thread_messages_received();
            if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
                error!(
                    "standalone io_handler: failed to send WriteResponse (VM input channel closed)"
                );
            }
            return;
        },
    };

    // Forward to consumer (terminal stdout, HTTP gateway, etc.). Use send().await to apply
    // back-pressure when the channel is full, preventing silent data loss that could cause
    // intermittent output mismatches.
    if output_tx.send(data).await.is_err() {
        trace!("standalone io_handler: output channel closed, discarding write data");
        let response: Message =
            WriteResponse::build(tid, -1, ProcessIdentifier::KERNEL, MessageType::Ikc);
        counters.increment_io_thread_messages_received();
        if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
            error!("standalone io_handler: failed to send WriteResponse (VM input channel closed)");
        }
        return;
    }

    // Send WriteResponse back to guest.
    let response: Message =
        WriteResponse::build(tid, written, ProcessIdentifier::KERNEL, MessageType::Ikc);
    trace!("standalone io_handler: sending WriteResponse (written={written}, tid={tid:?})");
    counters.increment_io_thread_messages_received();
    if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
        error!("standalone io_handler: failed to send WriteResponse (VM input channel closed)");
    }
}

///
/// # Description
///
/// Handles a guest ReadRequest by consuming the pull-header bulk frame, reading data from the
/// external input channel, and sending a bulk data response followed by a ReadResponse back to
/// the guest.
///
#[allow(clippy::too_many_arguments)]
async fn handle_read_request(
    vm_stdout_rx: &mut mpsc::Receiver<IkcFrame>,
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    input_rx: &mut mpsc::Receiver<Vec<u8>>,
    input_buffer: &mut VecDeque<u8>,
    input_closed: &mut bool,
    tid: ThreadIdentifier,
    request: &ReadRequest,
    counters: &MessageCounters,
) {
    let fd: i32 = request.fd;
    trace!("standalone io_handler: handling ReadRequest (fd={fd}, tid={tid:?})");

    // Wait for the pull-header bulk frame. The kernel emits this when the guest calls
    // ipc::pull(). The header contains the guest buffer address and maximum byte count.
    let pull_header: DataChunkHeader = match vm_stdout_rx.recv().await {
        Some(IkcFrame::Bulk(bulk)) => *bulk.header(),
        other => {
            error!(
                "standalone io_handler: expected bulk frame after ReadRequest, got {:?}",
                other.as_ref().map(|f| f.frame_type_byte())
            );
            let response: Message =
                ReadResponse::eof(tid, ProcessIdentifier::KERNEL, MessageType::Ikc);
            counters.increment_io_thread_messages_received();
            if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
                error!(
                    "standalone io_handler: failed to send ReadResponse (VM input channel closed)"
                );
            }
            return;
        },
    };

    // Only bridge reads from stdin; reject other FDs.
    if fd != STDIN_FILENO {
        warn!("standalone io_handler: rejecting read from unsupported fd={fd} (tid={tid:?})");
        // Send an empty bulk response and an error ReadResponse to satisfy the pull protocol.
        let error_header: DataChunkHeader = DataChunkHeader::new(
            pull_header.source_pid(),
            pull_header.source_tid(),
            pull_header.destination_pid(),
            pull_header.destination_tid(),
            pull_header.data_addr(),
            0,
        );
        let error_bulk: DataChunk = DataChunk::new(error_header, Vec::new());
        counters.increment_io_thread_messages_received();
        counters.increment_io_thread_messages_received();
        if vm_stdin_tx.send(IkcFrame::Bulk(error_bulk)).await.is_err() {
            error!("standalone io_handler: failed to send bulk response (VM input channel closed)");
            return;
        }
        let empty_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
        let response: Message =
            ReadResponse::build(tid, -1, empty_buf, ProcessIdentifier::KERNEL, MessageType::Ikc);
        if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
            error!("standalone io_handler: failed to send ReadResponse (VM input channel closed)");
        }
        return;
    }

    let max_bytes: usize = core::cmp::min(request.count as usize, pull_header.data_len() as usize);

    // If the internal buffer is empty and the input channel is still open, wait for data.
    if input_buffer.is_empty() && !*input_closed {
        match input_rx.recv().await {
            Some(data) => input_buffer.extend(data),
            None => {
                *input_closed = true;
            },
        }
    }

    // Take up to max_bytes from the buffer.
    let available: usize = core::cmp::min(input_buffer.len(), max_bytes);
    let data: Vec<u8> = input_buffer.drain(..available).collect();
    let actual_len: u32 = match u32::try_from(data.len()) {
        Ok(n) => n,
        Err(_) => {
            error!("standalone io_handler: read size overflows u32 (len={})", data.len());
            let empty_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
            let response: Message = ReadResponse::build(
                tid,
                -1,
                empty_buf,
                ProcessIdentifier::KERNEL,
                MessageType::Ikc,
            );
            counters.increment_io_thread_messages_received();
            if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
                error!(
                    "standalone io_handler: failed to send ReadResponse (VM input channel closed)"
                );
            }
            return;
        },
    };

    // Construct bulk response with the read data and send it to the guest. The input_fn will
    // write this data to guest memory at the pull_header's data_addr and construct a
    // PullResponse notification to wake the sleeping pull thread.
    let response_header: DataChunkHeader = DataChunkHeader::new(
        pull_header.source_pid(),
        pull_header.source_tid(),
        pull_header.destination_pid(),
        pull_header.destination_tid(),
        pull_header.data_addr(),
        actual_len,
    );
    let response_bulk: DataChunk = DataChunk::new(response_header, data);

    // Increment once for the bulk frame and once for the message response that follow.
    counters.increment_io_thread_messages_received();
    counters.increment_io_thread_messages_received();
    if vm_stdin_tx
        .send(IkcFrame::Bulk(response_bulk))
        .await
        .is_err()
    {
        error!("standalone io_handler: failed to send bulk response (VM input channel closed)");
        return;
    }

    // Send ReadResponse to guest. The empty buffer is expected — actual data was already
    // transferred via the bulk frame above.
    let empty_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
    let response: Message = ReadResponse::build(
        tid,
        actual_len.cast_signed(),
        empty_buf,
        ProcessIdentifier::KERNEL,
        MessageType::Ikc,
    );
    if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
        error!("standalone io_handler: failed to send ReadResponse (VM input channel closed)");
    }
}
