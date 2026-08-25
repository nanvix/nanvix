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
use ::nanvix_sandbox_config::{
    HostFilter,
    NetworkingMode,
};
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
        RequestIdentifier,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    SystemCallMessage,
    SystemCallMessageKind,
    message::SystemCallMessagePart,
    poll::input_message::{
        PollInputRequest,
        PollInputResponse,
    },
    unistd::message::{
        ReadRequest,
        ReadResponse,
        WriteRequest,
        WriteResponse,
    },
};
use ::tokio::{
    sync::mpsc,
    task::{
        AbortHandle,
        JoinHandle,
    },
    time::{
        Duration,
        Instant,
        sleep,
    },
};

#[cfg(feature = "profile-time")]
use crate::perf::PerfTimings;

//==================================================================================================
// Type Aliases
//==================================================================================================

/// Payload sent to the hostfsd worker thread: the IKC message, a channel for the
/// response, and the shared message counters.
type HostFsRequest = (Message, mpsc::Sender<IkcFrame>, MessageCounters);

/// Request handled by the asynchronous host-console input broker.
enum ConsoleInputRequest {
    /// Stops the broker and releases its VM input sender.
    Shutdown,
    /// Enables unsolicited input-availability notifications to VFSD.
    Subscribe,
    /// A blocking console read, completed when input or EOF becomes available.
    Read {
        tid: ThreadIdentifier,
        response_context: ConsoleResponseContext,
        pull_header: DataChunkHeader,
        max_bytes: usize,
    },
    /// An immediate console readiness snapshot.
    Poll {
        source_pid: ProcessIdentifier,
        pull_header: DataChunkHeader,
        max_bytes: usize,
    },
    /// An immediate readiness snapshot returned inline without a bulk transfer.
    PollStatus {
        source_pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        response_context: ConsoleResponseContext,
    },
}

//==================================================================================================
// Constants
//==================================================================================================

/// Standard POSIX file descriptors for FD validation in I/O handlers.
const STDIN_FILENO: i32 = 0;
const STDOUT_FILENO: i32 = 1;
const STDERR_FILENO: i32 = 2;

/// Interval at which the shutdown watchdog polls for guest VM completion.
const WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Grace period granted to the I/O handler to drain any remaining guest output after the guest
/// VM has exited, before it is force-aborted to unblock consumers waiting on the output channel.
const IO_HANDLER_SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

//==================================================================================================
// Structures
//==================================================================================================

/// Routing metadata for a direct console syscall response.
#[derive(Clone, Copy)]
struct ConsoleResponseContext {
    destination: MessageReceiver,
    request_id: RequestIdentifier,
}

impl ConsoleResponseContext {
    fn new(source: MessageSender, request_id: RequestIdentifier) -> Self {
        Self {
            destination: MessageReceiver::new(source.pid, source.tid),
            request_id,
        }
    }

    fn prepare(self, mut response: Message) -> Message {
        response.destination = self.destination;
        self.request_id.write_to(&mut response);
        response
    }

    fn source_pid(self) -> ProcessIdentifier {
        self.destination.pid
    }

    fn source_tid(self) -> ThreadIdentifier {
        self.destination.tid
    }
}

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
    /// Watchdog task that bounds the I/O handler's shutdown once the guest VM has exited, so a
    /// parked handler can never keep the output channel (and thus nanvixd) alive forever.
    watchdog_handle: JoinHandle<()>,
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
        host_filter: HostFilter,
        #[cfg(feature = "gdb")] gdb_port: Option<u16>,
    ) -> (Self, StandaloneVmIo) {
        // Create internal VM channels and wire them directly to the standalone I/O handler.
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
                host_filter,
                mount_directory,
            )
            .await;
        });

        // Shutdown watchdog: once the guest VM task finishes, the I/O handler must drain any
        // remaining guest output and then close so that consumers draining the output channel
        // (e.g., nanvix-terminal's `bridge_io`) observe EOF and stop waiting. If the handler does
        // not close within a grace period -- for instance because it is parked on an IKC frame or
        // on input that will never arrive now that the guest is gone -- it is force-aborted.
        // Aborting drops the handler's `output_tx`, which unblocks consumers and lets nanvixd
        // exit instead of hanging indefinitely. `abort_handle()` lets the watchdog observe and
        // abort the tasks without consuming the join handles that `wait()` awaits.
        let vmm_observer: AbortHandle = vmm_handle.abort_handle();
        let io_aborter: AbortHandle = io_handle.abort_handle();
        let watchdog_handle: JoinHandle<()> = tokio::spawn(async move {
            while !vmm_observer.is_finished() {
                sleep(WATCHDOG_POLL_INTERVAL).await;
            }
            // Give the I/O handler a chance to drain and exit on its own, polling so the watchdog
            // returns promptly on the healthy path while still bounding teardown by the grace
            // period when the handler is wedged.
            let deadline: Instant = Instant::now() + IO_HANDLER_SHUTDOWN_GRACE;
            while !io_aborter.is_finished() && Instant::now() < deadline {
                sleep(WATCHDOG_POLL_INTERVAL).await;
            }
            if !io_aborter.is_finished() {
                warn!(
                    "standalone: I/O handler still running {:?} after VM exit; forcing shutdown",
                    IO_HANDLER_SHUTDOWN_GRACE
                );
                io_aborter.abort();
            }
        });

        let handle: Self = Self {
            vmm_handle,
            io_handle,
            watchdog_handle,
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
        let join_result: ::std::result::Result<Result<u16>, ::tokio::task::JoinError> =
            self.vmm_handle.await;
        debug!("standalone: VM task settled");

        // Wait for the I/O handler to finish. The shutdown watchdog guarantees this resolves even
        // if the handler would otherwise park forever after the guest exits: the watchdog aborts
        // it after a grace period, which surfaces here as a cancellation rather than a hang.
        match self.io_handle.await {
            Ok(()) => {},
            Err(error) if error.is_cancelled() => {
                debug!("standalone: I/O handler task was cancelled during shutdown");
            },
            Err(error) => {
                warn!("standalone: I/O handler task failed (error={error:?})");
            },
        }

        // Both tasks have settled, so the watchdog is no longer needed.
        self.watchdog_handle.abort();

        // Emit performance timings to host stderr so the benchmark can parse them.
        #[cfg(feature = "profile-time")]
        self.perf_timings.emit_to_stderr();

        let vm_exit_status: Result<u16> = join_result?;
        debug!("standalone: VM completed (exit_status={vm_exit_status:?})");
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
        self.watchdog_handle.abort();
    }

    ///
    /// # Description
    ///
    /// Aborts then awaits both the VM and I/O handler tasks to ensure clean shutdown.
    ///
    pub async fn abort_and_wait(self) {
        self.vmm_handle.abort();
        self.io_handle.abort();
        self.watchdog_handle.abort();
        let _ = self.vmm_handle.await;
        let _ = self.io_handle.await;
        let _ = self.watchdog_handle.await;
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
/// The kernel stamps the originating thread into the
/// [`MessageSender::tid`](::sys::ipc::MessageSender) field of write/read requests, so it is read
/// directly here.
///
fn extract_tid(source: ::sys::ipc::MessageSender) -> ThreadIdentifier {
    let tid: ThreadIdentifier = source.tid;
    if tid.is_none() {
        warn!("standalone io_handler: message source has no thread id");
        return ThreadIdentifier::from(1i32);
    }
    tid
}

///
/// # Description
///
/// Processes guest IKC messages in standalone mode, bridging the VM's IKC channel to external
/// consumer channels.
///
/// This handler receives `WriteRequest` and `ReadRequest` messages from the guest via the VM's
/// stdout channel, processes them, and sends responses back via the VM's stdin channel.
/// Application data is forwarded to/from external channels used by terminal and HTTP consumers.
///
/// # Parameters
///
/// - `vm_stdout_rx`: Receives IKC frames emitted by the guest (via `output_fn`).
/// - `vm_stdin_tx`: Sends IKC frames to the guest (consumed by `input_fn`).
/// - `output_tx`: Forwards application data written by the guest to the external consumer.
/// - `input_rx`: Receives application data from the external consumer for guest reads.
///
#[allow(clippy::too_many_arguments)]
async fn standalone_io_handler(
    mut vm_stdout_rx: mpsc::Receiver<IkcFrame>,
    vm_stdin_tx: mpsc::Sender<IkcFrame>,
    output_tx: mpsc::Sender<Vec<u8>>,
    input_rx: mpsc::Receiver<Vec<u8>>,
    counters: MessageCounters,
    networking_mode: NetworkingMode,
    host_filter: HostFilter,
    mount_directory: Option<String>,
) {
    let console_input_tx: mpsc::Sender<ConsoleInputRequest> =
        spawn_console_input_broker(input_rx, vm_stdin_tx.downgrade(), counters.clone());

    // Tracks the logical op_id of the long-request multi-part stream currently
    // being forwarded to the hostfsd worker. Captured on part 0 and cleared once
    // the last part is forwarded (or as soon as an enqueue failure causes us to
    // emit a synthetic error response). Parts of distinct long requests do not
    // interleave on the IKC channel (vfsd sends them sequentially), so a single
    // slot is sufficient. Used only on the worker-channel-full recovery path:
    // if part 0 enqueues successfully but a later part fails, we can still echo
    // the logical op_id back so vfsd's pending-op table is drained instead of
    // leaving the originating syscall stuck.
    let mut worker_long_op_id: Option<::hostfs_api::OperationId> = None;

    let network_daemon: Option<Arc<NetworkDaemon>> = if networking_mode.is_enabled() {
        match NetworkDaemon::new(host_filter) {
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
                            let response_payload = match handler.handle_request(&msg.payload) {
                                Some(payload) => payload,
                                None => {
                                    // Intermediate multi-part message; no response yet.
                                    continue;
                                },
                            };
                            // Build the first response message and any queued
                            // multi-part follow-ups together so the entire response
                            // stream is delivered before the next request is
                            // processed. Hostfs multi-part responses (the long-target
                            // `readlink` and long-name `readdir` forms) leave
                            // additional payloads in the handler's queue that must be
                            // drained via `take_next_response_part` immediately after
                            // the head.
                            let mut response_payloads: std::vec::Vec<[u8; Message::PAYLOAD_SIZE]> =
                                std::vec::Vec::new();
                            response_payloads.push(response_payload);
                            while let Some(extra) = handler.take_next_response_part() {
                                response_payloads.push(extra);
                            }
                            let mut send_failed = false;
                            for payload in response_payloads {
                                let response: Message = Message::new(
                                    MessageSender::KERNEL,
                                    MessageReceiver::VFSD,
                                    MessageType::Ikc,
                                    None,
                                    payload,
                                );
                                counters.increment_io_handler_messages_sent();
                                if response_tx
                                    .blocking_send(IkcFrame::Message(response))
                                    .is_err()
                                {
                                    error!(
                                        "hostfsd-worker: failed to send response (VM input \
                                         channel closed)"
                                    );
                                    send_failed = true;
                                    break;
                                }
                            }
                            if send_failed {
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

                let header = syscall_msg.kind();
                match header {
                    SystemCallMessageKind::WriteRequest => {
                        let source: MessageSender = msg.source;
                        let tid: ThreadIdentifier = extract_tid(source);
                        let response_context: ConsoleResponseContext =
                            ConsoleResponseContext::new(source, syscall_msg.request_id());
                        let req: WriteRequest = WriteRequest::from_bytes(syscall_msg.payload);
                        handle_write_request(
                            &mut vm_stdout_rx,
                            &vm_stdin_tx,
                            &output_tx,
                            tid,
                            response_context,
                            &req,
                            &counters,
                        )
                        .await;
                    },
                    SystemCallMessageKind::ReadRequest => {
                        let source: MessageSender = msg.source;
                        let tid: ThreadIdentifier = extract_tid(source);
                        let response_context: ConsoleResponseContext =
                            ConsoleResponseContext::new(source, syscall_msg.request_id());
                        let req: ReadRequest = ReadRequest::from_bytes(syscall_msg.payload);
                        handle_read_request(
                            &mut vm_stdout_rx,
                            &vm_stdin_tx,
                            &console_input_tx,
                            tid,
                            response_context,
                            &req,
                            &counters,
                        )
                        .await;
                    },
                    SystemCallMessageKind::PollInputRequest => {
                        let source: MessageSender = msg.source;
                        let response_context: ConsoleResponseContext =
                            ConsoleResponseContext::new(source, syscall_msg.request_id());
                        let request: PollInputRequest =
                            PollInputRequest::from_bytes(syscall_msg.payload);
                        handle_poll_input_request(
                            &mut vm_stdout_rx,
                            &vm_stdin_tx,
                            &console_input_tx,
                            response_context,
                            &request,
                            &counters,
                        )
                        .await;
                    },
                    SystemCallMessageKind::ConsoleInputSubscribe => {
                        let source: MessageSender = msg.source;
                        if source.pid != ProcessIdentifier::VFSD {
                            warn!(
                                "standalone io_handler: rejecting console subscription from {:?}",
                                source.pid
                            );
                            continue;
                        }
                        if console_input_tx
                            .send(ConsoleInputRequest::Subscribe)
                            .await
                            .is_err()
                        {
                            error!("standalone io_handler: console input broker is unavailable");
                        }
                    },
                    SystemCallMessageKind::SendSocketRequest => {
                        handle_send_request(
                            &mut vm_stdout_rx,
                            &vm_stdin_tx,
                            &network_daemon,
                            msg.source,
                            syscall_msg,
                            &counters,
                        )
                        .await;
                    },
                    SystemCallMessageKind::SendToSocketRequest => {
                        handle_sendto_request(
                            &mut vm_stdout_rx,
                            &vm_stdin_tx,
                            &network_daemon,
                            msg.source,
                            syscall_msg,
                            &counters,
                        )
                        .await;
                    },
                    SystemCallMessageKind::ReceiveFromSocketRequest => {
                        handle_recvfrom_request(
                            &mut vm_stdout_rx,
                            &vm_stdin_tx,
                            &network_daemon,
                            msg.source,
                            syscall_msg,
                            &counters,
                        )
                        .await;
                    },
                    SystemCallMessageKind::ReceiveSocketRequest => {
                        handle_recv_request(
                            &mut vm_stdout_rx,
                            &vm_stdin_tx,
                            &network_daemon,
                            msg.source,
                            syscall_msg,
                            &counters,
                        )
                        .await;
                    },
                    header if header.is_hostfs() => {
                        // For multi-part long requests, vfsd allocates a single pending
                        // entry keyed on the logical op_id but emits N SystemCallMessagePart
                        // frames. If we naively responded once per fragment using the op_id
                        // field read from Message.payload[2..6], vfsd could not match a request
                        // whose outer identifier is absent or malformed and the originating
                        // syscall would block forever.
                        //
                        // Resolve the logical op_id and decide whether this frame should
                        // produce an error response on the no-worker / channel-full paths.
                        // Long-request parts: only respond once (on part 0) using the
                        // logical op_id embedded in the assembled wire payload.
                        let error_target: Option<::hostfs_api::OperationId> =
                            hostfs_error_target(header, &msg.payload, &syscall_msg.payload);

                        // Capture part metadata when this is a request-part header so the
                        // worker path can track multi-part progress.
                        let part_info: Option<(u16, u16)> =
                            hostfs_part_info(header, &syscall_msg.payload);

                        // On part 0 of a long request, remember the logical op_id so we
                        // can still emit a matching error response if a later part of the
                        // same stream fails to enqueue into the worker channel.
                        if let Some((0, _)) = part_info {
                            worker_long_op_id = error_target;
                        }

                        if let Some(ref tx) = hostfs_tx {
                            if tx
                                .send((msg, vm_stdin_tx.clone(), counters.clone()))
                                .is_err()
                            {
                                error!("standalone io_handler: hostfs worker channel closed");
                                // For single-message requests and part 0 of long requests,
                                // `error_target` carries the logical op_id. For non-first
                                // parts of a long request whose part 0 already entered the
                                // worker before it closed, fall back to the cached
                                // `worker_long_op_id` so vfsd's pending entry is still drained.
                                let effective_target: Option<::hostfs_api::OperationId> =
                                    error_target.or(worker_long_op_id);
                                if let Some(op_id) = effective_target {
                                    send_hostfs_error(header, op_id, &vm_stdin_tx, &counters).await;
                                }
                                // Drop tracking after emitting the error so subsequent
                                // parts of this stranded stream are silently dropped and
                                // do not interfere with the next long request.
                                worker_long_op_id = None;
                            } else if let Some((part_number, total_parts)) = part_info {
                                // Stream forwarded cleanly: clear tracking once the last
                                // part has been handed off to the worker. Use checked
                                // arithmetic because `part_number` and `total_parts` come
                                // from a guest-controlled frame: an attacker (or a
                                // malformed frame) could set `part_number == u16::MAX`,
                                // which would panic on overflow in debug builds with a
                                // plain `+ 1`. A frame with `total_parts == 0` or with
                                // `part_number + 1` overflowing simply fails the equality
                                // and leaves tracking in place; the next valid part 0
                                // overwrites it, and any genuinely stranded entry is
                                // cleared by the channel-full error path above.
                                if part_number.checked_add(1) == Some(total_parts) {
                                    worker_long_op_id = None;
                                }
                            }
                            continue;
                        }
                        // No mount directory configured: send an error response so
                        // vfsd can drain its pending queue and report the error to
                        // the caller instead of leaving them blocked.
                        if let Some(op_id) = error_target {
                            warn!(
                                "standalone io_handler: hostfs message received but no mount \
                                 configured; sending error response"
                            );
                            send_hostfs_error(header, op_id, &vm_stdin_tx, &counters).await;
                        } else {
                            trace!(
                                "standalone io_handler: dropping non-first hostfs request part \
                                 (no mount configured); error response already emitted on part 0"
                            );
                        }
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
                            let response_context: ConsoleResponseContext =
                                ConsoleResponseContext::new(msg.source, syscall_msg.request_id());
                            send_networking_error(&vm_stdin_tx, response_context, &counters).await;
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

    let _ = console_input_tx.send(ConsoleInputRequest::Shutdown).await;
    debug!("standalone: I/O handler exiting (VM stdout channel closed)");
}

///
/// # Description
///
/// Determines whether a hostfs request should produce an error response on the
/// no-host-worker / channel-full paths, and returns the logical [`OperationId`]
/// to echo back.
///
/// For single-message hostfs requests, the op_id is encoded at bytes `2..6` of
/// the `Message` payload (which is `syscall_msg.payload[0..4]`) and a response
/// is always emitted.
///
/// For multi-part long requests (`HostFs*RequestPart`), vfsd emits one
/// `SystemCallMessagePart` frame per chunk but allocates only a single pending
/// entry keyed on the logical op_id of the assembled request. Responding once
/// per fragment using the per-part header bytes as the op_id would orphan every
/// response (vfsd's recv loop matches none of them) and hang the caller. To
/// avoid this, only part 0 produces an error response, carrying the logical
/// op_id read from the first 4 bytes of the assembled long-message wire payload
/// (which live at the start of part 0's chunk). Subsequent parts are dropped.
///
/// Returns `Some(op_id)` if an error response should be emitted, or `None` if
/// the frame should be silently dropped (non-first long-request part).
///
fn hostfs_error_target(
    header: SystemCallMessageKind,
    message_payload: &[u8; Message::PAYLOAD_SIZE],
    syscall_payload: &[u8; SystemCallMessage::PAYLOAD_SIZE],
) -> Option<::hostfs_api::OperationId> {
    let is_request_part: bool = matches!(
        header,
        SystemCallMessageKind::HostFsOpenRequestPart
            | SystemCallMessageKind::HostFsRenameRequestPart
            | SystemCallMessageKind::HostFsUnlinkRequestPart
            | SystemCallMessageKind::HostFsMkdirRequestPart
            | SystemCallMessageKind::HostFsRmdirRequestPart
            | SystemCallMessageKind::HostFsSymlinkRequestPart
            | SystemCallMessageKind::HostFsReadlinkRequestPart
            | SystemCallMessageKind::HostFsLstatRequestPart
            | SystemCallMessageKind::HostFsPathStatRequestPart
    );

    if is_request_part {
        let part: SystemCallMessagePart = SystemCallMessagePart::from_bytes(*syscall_payload);
        // Only the first fragment of a long request produces an error response.
        // The logical op_id of the assembled wire payload sits in the first 4
        // bytes of part 0's chunk (all long-message wire formats start with
        // `[op_id:4]`, little-endian).
        if { part.part_number } != 0 {
            return None;
        }
        // Defensively validate the declared payload size before reading the
        // logical op_id. `part.payload` is a fixed-size array so indexing
        // `[0..4]` cannot panic, but a malformed frame with `payload_size < 4`
        // would expose trailing zero bytes (or stale buffer contents) as if
        // they were the op_id. In that case fall back to `OperationId::INVALID`
        // so vfsd's `PendingQueue::remove` simply drops the response instead of
        // matching the wrong pending entry.
        let declared: usize = { part.payload_size } as usize;
        if declared < ::core::mem::size_of::<::hostfs_api::OperationId>() {
            warn!(
                "standalone io_handler: malformed hostfs request part (payload_size={declared} < \
                 4); using OperationId::INVALID for error response"
            );
            return Some(::hostfs_api::OperationId::INVALID);
        }
        let chunk: &[u8] = &part.payload;
        Some(::hostfs_api::OperationId::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
    } else {
        // Single-message hostfs request: the op_id is at Message.payload[2..6].
        Some(::hostfs_api::get_op_id(message_payload))
    }
}

///
/// # Description
///
/// Returns `(part_number, total_parts)` if `header` is a long-request part frame,
/// or `None` otherwise.
///
/// Used by the worker-channel-full recovery path to track multi-part progress so
/// that a later-part enqueue failure can still emit a matching error response
/// using the op_id cached from part 0.
///
fn hostfs_part_info(
    header: SystemCallMessageKind,
    syscall_payload: &[u8; SystemCallMessage::PAYLOAD_SIZE],
) -> Option<(u16, u16)> {
    let is_request_part: bool = matches!(
        header,
        SystemCallMessageKind::HostFsOpenRequestPart
            | SystemCallMessageKind::HostFsRenameRequestPart
            | SystemCallMessageKind::HostFsUnlinkRequestPart
            | SystemCallMessageKind::HostFsMkdirRequestPart
            | SystemCallMessageKind::HostFsRmdirRequestPart
            | SystemCallMessageKind::HostFsSymlinkRequestPart
            | SystemCallMessageKind::HostFsReadlinkRequestPart
            | SystemCallMessageKind::HostFsLstatRequestPart
            | SystemCallMessageKind::HostFsPathStatRequestPart
    );
    if !is_request_part {
        return None;
    }
    let part: SystemCallMessagePart = SystemCallMessagePart::from_bytes(*syscall_payload);
    Some(({ part.part_number }, { part.total_parts }))
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
    header: SystemCallMessageKind,
    op_id: ::hostfs_api::OperationId,
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    counters: &MessageCounters,
) {
    let resp_header: SystemCallMessageKind = match header.hostfs_response_kind() {
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
        SystemCallMessageKind::HostFsLseekResponse => {
            // Lseek completion checks offset as i64 < 0.
            err_payload[ds..ds + 8]
                .copy_from_slice(&(::hostfs_api::HOSTFS_ERR_IO as i64).to_le_bytes());
        },
        SystemCallMessageKind::HostFsStatResponse
        | SystemCallMessageKind::HostFsReadDirResponse => {
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
        MessageSender::KERNEL,
        MessageReceiver::VFSD,
        MessageType::Ikc,
        None,
        err_payload,
    );
    counters.increment_io_handler_messages_sent();
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
        Some(response) => {
            counters.increment_io_handler_messages_sent();
            if vm_stdin_tx
                .blocking_send(IkcFrame::Message(response))
                .is_err()
            {
                error!(
                    "standalone io_handler: failed to send networking response (VM input channel \
                     closed)"
                );
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
    response_context: ConsoleResponseContext,
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
            let response: Message = response_context.prepare(WriteResponse::build(
                tid,
                0,
                ProcessIdentifier::KERNEL,
                MessageType::Ikc,
            ));
            counters.increment_io_handler_messages_sent();
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
        let response: Message = response_context.prepare(WriteResponse::build(
            tid,
            -1,
            ProcessIdentifier::KERNEL,
            MessageType::Ikc,
        ));
        counters.increment_io_handler_messages_sent();
        if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
            error!("standalone io_handler: failed to send WriteResponse (VM input channel closed)");
        }
        return;
    }

    let written: i32 = match i32::try_from(data.len()) {
        Ok(n) => n,
        Err(_) => {
            error!("standalone io_handler: write size overflows i32 (len={})", data.len());
            let response: Message = response_context.prepare(WriteResponse::build(
                tid,
                -1,
                ProcessIdentifier::KERNEL,
                MessageType::Ikc,
            ));
            counters.increment_io_handler_messages_sent();
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
        let response: Message = response_context.prepare(WriteResponse::build(
            tid,
            -1,
            ProcessIdentifier::KERNEL,
            MessageType::Ikc,
        ));
        counters.increment_io_handler_messages_sent();
        if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
            error!("standalone io_handler: failed to send WriteResponse (VM input channel closed)");
        }
        return;
    }

    // Send WriteResponse back to guest.
    let response: Message = response_context.prepare(WriteResponse::build(
        tid,
        written,
        ProcessIdentifier::KERNEL,
        MessageType::Ikc,
    ));
    trace!("standalone io_handler: sending WriteResponse (written={written}, tid={tid:?})");
    counters.increment_io_handler_messages_sent();
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
    console_input_tx: &mpsc::Sender<ConsoleInputRequest>,
    tid: ThreadIdentifier,
    response_context: ConsoleResponseContext,
    request: &ReadRequest,
    counters: &MessageCounters,
) {
    let fd: i32 = request.fd;
    trace!("standalone io_handler: handling ReadRequest (fd={fd}, tid={tid:?})");

    // Wait for the pull-header bulk frame. The kernel emits this when the guest calls
    // ipc::pull(). The header carries an opaque bulk location and maximum byte count; the location
    // is a guest buffer address on legacy paths or a UserVM transfer id for scatter/gather pulls.
    let pull_header: DataChunkHeader = match vm_stdout_rx.recv().await {
        Some(IkcFrame::Bulk(bulk)) => *bulk.header(),
        other => {
            error!(
                "standalone io_handler: expected bulk frame after ReadRequest, got {:?}",
                other.as_ref().map(|f| f.frame_type_byte())
            );
            let response: Message = response_context.prepare(ReadResponse::eof(
                tid,
                ProcessIdentifier::KERNEL,
                MessageType::Ikc,
            ));
            counters.increment_io_handler_messages_sent();
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
        counters.increment_io_handler_messages_sent();
        counters.increment_io_handler_messages_sent();
        if vm_stdin_tx.send(IkcFrame::Bulk(error_bulk)).await.is_err() {
            error!("standalone io_handler: failed to send bulk response (VM input channel closed)");
            return;
        }
        let empty_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
        let response: Message = response_context.prepare(ReadResponse::build(
            tid,
            -1,
            empty_buf,
            ProcessIdentifier::KERNEL,
            MessageType::Ikc,
        ));
        if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
            error!("standalone io_handler: failed to send ReadResponse (VM input channel closed)");
        }
        return;
    }

    let max_bytes: usize = core::cmp::min(request.count as usize, pull_header.data_len() as usize);
    if console_input_tx
        .send(ConsoleInputRequest::Read {
            tid,
            response_context,
            pull_header,
            max_bytes,
        })
        .await
        .is_err()
    {
        error!("standalone io_handler: console input broker is unavailable");
        send_empty_pull_response(vm_stdin_tx, &pull_header, counters).await;
        let response: Message = response_context.prepare(ReadResponse::build(
            tid,
            -1,
            [0u8; ReadResponse::BUFFER_SIZE],
            ProcessIdentifier::KERNEL,
            MessageType::Ikc,
        ));
        counters.increment_io_handler_messages_sent();
        if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
            error!("standalone io_handler: failed to send ReadResponse (VM input channel closed)");
        }
    }
}

/// Handles an immediate, non-blocking host-console input snapshot.
async fn handle_poll_input_request(
    vm_stdout_rx: &mut mpsc::Receiver<IkcFrame>,
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    console_input_tx: &mpsc::Sender<ConsoleInputRequest>,
    response_context: ConsoleResponseContext,
    request: &PollInputRequest,
    counters: &MessageCounters,
) {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let tid: ThreadIdentifier = response_context.source_tid();
    if request.count() == 0 {
        if console_input_tx
            .send(ConsoleInputRequest::PollStatus {
                source_pid,
                tid,
                response_context,
            })
            .await
            .is_err()
        {
            error!("standalone io_handler: console input broker is unavailable");
            send_console_poll_status(
                vm_stdin_tx,
                tid,
                response_context,
                PollInputRequest::STATUS_EMPTY,
                counters,
            )
            .await;
        }
        return;
    }

    let pull_header: DataChunkHeader = match vm_stdout_rx.recv().await {
        Some(IkcFrame::Bulk(bulk)) => *bulk.header(),
        other => {
            error!(
                "standalone io_handler: expected bulk frame after PollInputRequest, got {:?}",
                other.as_ref().map(|frame| frame.frame_type_byte())
            );
            return;
        },
    };

    let max_response: usize = pull_header.data_len() as usize;
    let max_data: usize = core::cmp::min(request.count() as usize, max_response.saturating_sub(1));
    if console_input_tx
        .send(ConsoleInputRequest::Poll {
            source_pid,
            pull_header,
            max_bytes: max_data,
        })
        .await
        .is_err()
    {
        error!("standalone io_handler: console input broker is unavailable");
        send_empty_pull_response(vm_stdin_tx, &pull_header, counters).await;
    }
}

/// Starts the task that owns host-console input and completes reads without blocking the I/O loop.
fn spawn_console_input_broker(
    mut input_rx: mpsc::Receiver<Vec<u8>>,
    vm_stdin_tx: mpsc::WeakSender<IkcFrame>,
    counters: MessageCounters,
) -> mpsc::Sender<ConsoleInputRequest> {
    let (request_tx, mut request_rx): (
        mpsc::Sender<ConsoleInputRequest>,
        mpsc::Receiver<ConsoleInputRequest>,
    ) = mpsc::channel(CHANNEL_CAPACITY);

    tokio::spawn(async move {
        let mut input_buffer: VecDeque<u8> = VecDeque::new();
        let mut input_closed: bool = false;
        let mut notifications_enabled: bool = false;
        let mut notification_pending: bool = false;
        let mut pending_reads: VecDeque<(
            ThreadIdentifier,
            ConsoleResponseContext,
            DataChunkHeader,
            usize,
        )> = VecDeque::new();

        loop {
            while !pending_reads.is_empty()
                && (!input_buffer.is_empty()
                    || input_closed
                    || pending_reads
                        .front()
                        .map(|read| read.3 == 0)
                        .unwrap_or(false))
            {
                let Some((tid, response_context, pull_header, max_bytes)) =
                    pending_reads.pop_front()
                else {
                    break;
                };
                let available: usize = core::cmp::min(input_buffer.len(), max_bytes);
                let data: Vec<u8> = input_buffer.drain(..available).collect();
                let Some(sender) = vm_stdin_tx.upgrade() else {
                    return;
                };
                send_console_read_response(
                    &sender,
                    tid,
                    response_context,
                    pull_header,
                    data,
                    &counters,
                )
                .await;
            }

            tokio::select! {
                data = input_rx.recv(), if !input_closed
                    && (!pending_reads.is_empty()
                        || (notifications_enabled && input_buffer.is_empty())) => {
                    match data {
                        Some(data) => {
                            input_buffer.extend(data);
                            if notifications_enabled && !notification_pending {
                                let Some(sender) = vm_stdin_tx.upgrade() else {
                                    break;
                                };
                                if !notify_console_input_available(&sender, &counters).await {
                                    break;
                                }
                                notification_pending = true;
                            }
                        },
                        None => {
                            input_closed = true;
                            if notifications_enabled && !notification_pending {
                                let Some(sender) = vm_stdin_tx.upgrade() else {
                                    break;
                                };
                                if !notify_console_input_available(&sender, &counters).await {
                                    break;
                                }
                                notification_pending = true;
                            }
                        },
                    }
                },
                request = request_rx.recv() => {
                    let Some(request) = request else {
                        break;
                    };

                    let direct_input_request: bool = !notifications_enabled
                        && matches!(
                            &request,
                            ConsoleInputRequest::Read { .. }
                                | ConsoleInputRequest::Poll { .. }
                                | ConsoleInputRequest::PollStatus { .. }
                        );
                    if direct_input_request
                        && input_buffer.is_empty()
                        && let Ok(data) = input_rx.try_recv()
                    {
                        input_buffer.extend(data);
                    }
                    if direct_input_request && input_rx.is_closed() && input_rx.is_empty() {
                        input_closed = true;
                    }

                    match request {
                        ConsoleInputRequest::Shutdown => break,
                        ConsoleInputRequest::Subscribe => {
                            notifications_enabled = true;
                            if (!input_buffer.is_empty() || input_closed)
                                && !notification_pending {
                                let Some(sender) = vm_stdin_tx.upgrade() else {
                                    break;
                                };
                                if !notify_console_input_available(&sender, &counters).await {
                                    break;
                                }
                                notification_pending = true;
                            }
                        },
                        ConsoleInputRequest::Read {
                            tid,
                            response_context,
                            pull_header,
                            max_bytes,
                        } => {
                            pending_reads.push_back((
                                tid,
                                response_context,
                                pull_header,
                                max_bytes,
                            ));
                        },
                        ConsoleInputRequest::Poll {
                            source_pid,
                            pull_header,
                            max_bytes,
                        } => {
                            let from_vfsd: bool = source_pid == ProcessIdentifier::VFSD;
                            let authorized: bool = !notifications_enabled || from_vfsd;
                            let acknowledge_notification: bool = from_vfsd && notification_pending;
                            if acknowledge_notification {
                                notification_pending = false;
                            }
                            let mut data: Vec<u8> = Vec::with_capacity(max_bytes + 1);
                            if !authorized {
                                data.push(PollInputRequest::STATUS_EMPTY);
                            } else if input_buffer.is_empty() {
                                data.push(if input_closed {
                                    PollInputRequest::STATUS_EOF
                                } else {
                                    PollInputRequest::STATUS_EMPTY
                                });
                            } else {
                                data.push(PollInputRequest::STATUS_DATA);
                                let available: usize =
                                    core::cmp::min(input_buffer.len(), max_bytes);
                                data.extend(input_buffer.drain(..available));
                            }
                            let Some(sender) = vm_stdin_tx.upgrade() else {
                                break;
                            };
                            send_console_poll_response(
                                &sender,
                                pull_header,
                                data,
                                &counters,
                            )
                            .await;
                            if acknowledge_notification
                                && notifications_enabled
                                && !input_buffer.is_empty()
                                && !notification_pending
                            {
                                if !notify_console_input_available(&sender, &counters).await {
                                    break;
                                }
                                notification_pending = true;
                            }
                        },
                        ConsoleInputRequest::PollStatus {
                            source_pid,
                            tid,
                            response_context,
                        } => {
                            let authorized: bool = !notifications_enabled
                                || source_pid == ProcessIdentifier::VFSD;
                            let status: u8 = if !authorized {
                                PollInputRequest::STATUS_EMPTY
                            } else if input_buffer.is_empty() {
                                if input_closed {
                                    PollInputRequest::STATUS_EOF
                                } else {
                                    PollInputRequest::STATUS_EMPTY
                                }
                            } else {
                                PollInputRequest::STATUS_DATA
                            };
                            let Some(sender) = vm_stdin_tx.upgrade() else {
                                break;
                            };
                            send_console_poll_status(
                                &sender,
                                tid,
                                response_context,
                                status,
                                &counters,
                            )
                            .await;
                        },
                    }
                },
            }
        }
    });

    request_tx
}

/// Notifies VFSD that an immediate console-input snapshot can make progress.
async fn notify_console_input_available(
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    counters: &MessageCounters,
) -> bool {
    counters.increment_io_handler_messages_sent();
    let notification: Message = PollInputRequest::build_available_notification();
    if vm_stdin_tx
        .send(IkcFrame::Message(notification))
        .await
        .is_err()
    {
        error!("standalone io_handler: failed to notify VFSD of console input");
        false
    } else {
        true
    }
}

/// Sends the bulk payload and acknowledgement for a completed console read.
async fn send_console_read_response(
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    tid: ThreadIdentifier,
    response_context: ConsoleResponseContext,
    pull_header: DataChunkHeader,
    data: Vec<u8>,
    counters: &MessageCounters,
) {
    let actual_len: u32 = match u32::try_from(data.len()) {
        Ok(actual_len) => actual_len,
        Err(_) => {
            error!("standalone io_handler: console read length overflows u32");
            return;
        },
    };
    let response_header: DataChunkHeader = DataChunkHeader::new(
        pull_header.source_pid(),
        pull_header.source_tid(),
        pull_header.destination_pid(),
        pull_header.destination_tid(),
        pull_header.data_addr(),
        actual_len,
    );
    counters.increment_io_handler_messages_sent();
    counters.increment_io_handler_messages_sent();
    if vm_stdin_tx
        .send(IkcFrame::Bulk(DataChunk::new(response_header, data)))
        .await
        .is_err()
    {
        error!("standalone io_handler: failed to send console read bulk response");
        return;
    }

    let response: Message = response_context.prepare(ReadResponse::build(
        tid,
        actual_len.cast_signed(),
        [0u8; ReadResponse::BUFFER_SIZE],
        ProcessIdentifier::KERNEL,
        MessageType::Ikc,
    ));
    if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
        error!("standalone io_handler: failed to send console read response");
    }
}

/// Sends an immediate console input snapshot through the pending pull.
async fn send_console_poll_response(
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    pull_header: DataChunkHeader,
    data: Vec<u8>,
    counters: &MessageCounters,
) {
    let data_len: u32 = match u32::try_from(data.len()) {
        Ok(data_len) => data_len,
        Err(_) => {
            error!("standalone io_handler: poll-input response length overflows u32");
            return;
        },
    };
    let response_header: DataChunkHeader = DataChunkHeader::new(
        pull_header.source_pid(),
        pull_header.source_tid(),
        pull_header.destination_pid(),
        pull_header.destination_tid(),
        pull_header.data_addr(),
        data_len,
    );
    counters.increment_io_handler_messages_sent();
    if vm_stdin_tx
        .send(IkcFrame::Bulk(DataChunk::new(response_header, data)))
        .await
        .is_err()
    {
        error!("standalone io_handler: failed to send poll-input bulk response");
    }
}

/// Sends an immediate console readiness status without a bulk transfer.
async fn send_console_poll_status(
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    tid: ThreadIdentifier,
    response_context: ConsoleResponseContext,
    status: u8,
    counters: &MessageCounters,
) {
    counters.increment_io_handler_messages_sent();
    let response: Message = response_context.prepare(PollInputResponse::build(tid, status));
    if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
        error!("standalone io_handler: failed to send poll-input status response");
    }
}

///
/// # Description
///
/// Handles a guest `SendSocketRequest` by consuming the subsequent push data frame and forwarding
/// the payload to networkd on a blocking task, which sends the response back to the guest.
///
/// The push data frame must always be drained, even when networking is disabled, otherwise the
/// IKC frame stream desynchronizes and subsequent requests are misinterpreted.
///
async fn handle_send_request(
    vm_stdout_rx: &mut mpsc::Receiver<IkcFrame>,
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    network_daemon: &Option<Arc<NetworkDaemon>>,
    source: MessageSender,
    syscall_msg: SystemCallMessage,
    counters: &MessageCounters,
) {
    let tid: ThreadIdentifier = extract_tid(source);
    let response_context: ConsoleResponseContext =
        ConsoleResponseContext::new(source, syscall_msg.request_id());
    trace!("standalone io_handler: handling SendSocketRequest (tid={tid:?})");

    // Wait for the push data frame that the guest's `ipc::push()` emits after the request.
    let data: Vec<u8> = match vm_stdout_rx.recv().await {
        Some(IkcFrame::Bulk(bulk)) => bulk.into_data(),
        other => {
            error!(
                "standalone io_handler: expected bulk frame after SendSocketRequest, got {:?}",
                other.as_ref().map(|f| f.frame_type_byte())
            );
            send_networking_error(vm_stdin_tx, response_context, counters).await;
            return;
        },
    };

    let network_daemon: Arc<NetworkDaemon> = match network_daemon {
        Some(nd) => nd.clone(),
        None => {
            warn!("standalone io_handler: networking not allowed, rejecting send (tid={tid:?})");
            send_networking_error(vm_stdin_tx, response_context, counters).await;
            return;
        },
    };

    // Run the (potentially blocking) backend call on its own thread so it does not stall the I/O
    // handler loop, mirroring `spawn_networking_task`.
    let vm_stdin_tx: mpsc::Sender<IkcFrame> = vm_stdin_tx.clone();
    let counters: MessageCounters = counters.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let response: Message = network_daemon.handle_send(source, syscall_msg, &data);
        counters.increment_io_handler_messages_sent();
        if vm_stdin_tx
            .blocking_send(IkcFrame::Message(response))
            .is_err()
        {
            error!("standalone io_handler: failed to send send response (VM input channel closed)");
        }
    });

    tokio::spawn(async move {
        if let Err(e) = handle.await {
            error!("standalone io_handler: send task panicked: {e}");
        }
    });
}

///
/// # Description
///
/// Handles a guest `SendToSocketRequest` by consuming the subsequent push data frame and
/// forwarding the datagram to networkd on a blocking task, which sends the response back to the
/// guest.
///
/// The push data frame must always be drained — even when networking is disabled — otherwise the
/// IKC frame stream desynchronizes and subsequent requests are misinterpreted.
///
async fn handle_sendto_request(
    vm_stdout_rx: &mut mpsc::Receiver<IkcFrame>,
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    network_daemon: &Option<Arc<NetworkDaemon>>,
    source: MessageSender,
    syscall_msg: SystemCallMessage,
    counters: &MessageCounters,
) {
    let tid: ThreadIdentifier = extract_tid(source);
    let response_context: ConsoleResponseContext =
        ConsoleResponseContext::new(source, syscall_msg.request_id());
    trace!("standalone io_handler: handling SendToSocketRequest (tid={tid:?})");

    // Wait for the push data frame that the guest's `ipc::push()` emits after the request.
    let data: Vec<u8> = match vm_stdout_rx.recv().await {
        Some(IkcFrame::Bulk(bulk)) => bulk.into_data(),
        other => {
            error!(
                "standalone io_handler: expected bulk frame after SendToSocketRequest, got {:?}",
                other.as_ref().map(|f| f.frame_type_byte())
            );
            send_networking_error(vm_stdin_tx, response_context, counters).await;
            return;
        },
    };

    let network_daemon: Arc<NetworkDaemon> = match network_daemon {
        Some(nd) => nd.clone(),
        None => {
            warn!("standalone io_handler: networking not allowed, rejecting sendto (tid={tid:?})");
            send_networking_error(vm_stdin_tx, response_context, counters).await;
            return;
        },
    };

    // Run the (potentially blocking) backend call on its own thread so it does not stall the I/O
    // handler loop, mirroring `spawn_networking_task`.
    let vm_stdin_tx: mpsc::Sender<IkcFrame> = vm_stdin_tx.clone();
    let counters: MessageCounters = counters.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let response: Message = network_daemon.handle_sendto(source, syscall_msg, &data);
        counters.increment_io_handler_messages_sent();
        if vm_stdin_tx
            .blocking_send(IkcFrame::Message(response))
            .is_err()
        {
            error!(
                "standalone io_handler: failed to send sendto response (VM input channel closed)"
            );
        }
    });

    tokio::spawn(async move {
        if let Err(e) = handle.await {
            error!("standalone io_handler: sendto task panicked: {e}");
        }
    });
}

///
/// # Description
///
/// Handles a guest `ReceiveFromSocketRequest` by consuming the pull-header bulk frame and
/// forwarding the request to networkd on a blocking task, which pushes the received datagram back
/// to the guest followed by the response message.
///
async fn handle_recvfrom_request(
    vm_stdout_rx: &mut mpsc::Receiver<IkcFrame>,
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    network_daemon: &Option<Arc<NetworkDaemon>>,
    source: MessageSender,
    syscall_msg: SystemCallMessage,
    counters: &MessageCounters,
) {
    let tid: ThreadIdentifier = extract_tid(source);
    let response_context: ConsoleResponseContext =
        ConsoleResponseContext::new(source, syscall_msg.request_id());
    trace!("standalone io_handler: handling ReceiveFromSocketRequest (tid={tid:?})");

    // Wait for the pull-header bulk frame that the guest's `ipc::pull()` emits after the request.
    // It carries the bulk location the received datagram must be written to.
    let pull_header: DataChunkHeader = match vm_stdout_rx.recv().await {
        Some(IkcFrame::Bulk(bulk)) => *bulk.header(),
        other => {
            error!(
                "standalone io_handler: expected bulk frame after ReceiveFromSocketRequest, got \
                 {:?}",
                other.as_ref().map(|f| f.frame_type_byte())
            );
            send_networking_error(vm_stdin_tx, response_context, counters).await;
            return;
        },
    };

    let network_daemon: Arc<NetworkDaemon> = match network_daemon {
        Some(nd) => nd.clone(),
        None => {
            warn!(
                "standalone io_handler: networking not allowed, rejecting recvfrom (tid={tid:?})"
            );
            // Release the guest blocked in `ipc::pull()` with an empty transfer before reporting
            // the error so it does not deadlock.
            send_empty_pull_response(vm_stdin_tx, &pull_header, counters).await;
            send_networking_error(vm_stdin_tx, response_context, counters).await;
            return;
        },
    };

    // Run the blocking `recvfrom()` on its own thread so a waiting socket does not stall the I/O
    // handler loop.
    let vm_stdin_tx: mpsc::Sender<IkcFrame> = vm_stdin_tx.clone();
    let counters: MessageCounters = counters.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let (response, data): (Message, Vec<u8>) =
            network_daemon.handle_recvfrom(source, syscall_msg);

        // The datagram payload is bounded by the scatter/gather bulk limit. Guard the conversion
        // anyway, releasing the guest's pull with an empty transfer and an error on the unexpected
        // overflow so it does not deadlock.
        let actual_len: u32 = match u32::try_from(data.len()) {
            Ok(n) => n,
            Err(_) => {
                error!("standalone io_handler: recvfrom size overflows u32 (len={})", data.len());
                let empty_header: DataChunkHeader = DataChunkHeader::new(
                    pull_header.source_pid(),
                    pull_header.source_tid(),
                    pull_header.destination_pid(),
                    pull_header.destination_tid(),
                    pull_header.data_addr(),
                    0,
                );
                counters.increment_io_handler_messages_sent();
                counters.increment_io_handler_messages_sent();
                let _ = vm_stdin_tx
                    .blocking_send(IkcFrame::Bulk(DataChunk::new(empty_header, Vec::new())));
                let err: Message = response_context.prepare(Message::new(
                    MessageSender::NETWORKD,
                    MessageReceiver::KERNEL,
                    MessageType::Ikc,
                    Some(ErrorCode::InvalidMessage),
                    [0u8; Message::PAYLOAD_SIZE],
                ));
                let _ = vm_stdin_tx.blocking_send(IkcFrame::Message(err));
                return;
            },
        };
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
        counters.increment_io_handler_messages_sent();
        counters.increment_io_handler_messages_sent();
        if vm_stdin_tx
            .blocking_send(IkcFrame::Bulk(response_bulk))
            .is_err()
        {
            error!(
                "standalone io_handler: failed to send recvfrom bulk response (VM input channel \
                 closed)"
            );
            return;
        }
        if vm_stdin_tx
            .blocking_send(IkcFrame::Message(response))
            .is_err()
        {
            error!(
                "standalone io_handler: failed to send recvfrom response (VM input channel closed)"
            );
        }
    });

    tokio::spawn(async move {
        if let Err(e) = handle.await {
            error!("standalone io_handler: recvfrom task panicked: {e}");
        }
    });
}

///
/// # Description
///
/// Handles a guest `ReceiveSocketRequest` by consuming the pull-header bulk frame and forwarding
/// the request to networkd on a blocking task, which pushes the received payload back to the guest
/// followed by the response message.
///
async fn handle_recv_request(
    vm_stdout_rx: &mut mpsc::Receiver<IkcFrame>,
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    network_daemon: &Option<Arc<NetworkDaemon>>,
    source: MessageSender,
    syscall_msg: SystemCallMessage,
    counters: &MessageCounters,
) {
    let tid: ThreadIdentifier = extract_tid(source);
    let response_context: ConsoleResponseContext =
        ConsoleResponseContext::new(source, syscall_msg.request_id());
    trace!("standalone io_handler: handling ReceiveSocketRequest (tid={tid:?})");

    // Wait for the pull-header bulk frame that the guest's `ipc::pull()` emits after the request.
    // It carries the bulk location the received payload must be written to.
    let pull_header: DataChunkHeader = match vm_stdout_rx.recv().await {
        Some(IkcFrame::Bulk(bulk)) => *bulk.header(),
        other => {
            error!(
                "standalone io_handler: expected bulk frame after ReceiveSocketRequest, got {:?}",
                other.as_ref().map(|f| f.frame_type_byte())
            );
            send_networking_error(vm_stdin_tx, response_context, counters).await;
            return;
        },
    };

    let network_daemon: Arc<NetworkDaemon> = match network_daemon {
        Some(nd) => nd.clone(),
        None => {
            warn!("standalone io_handler: networking not allowed, rejecting recv (tid={tid:?})");
            // Release the guest blocked in `ipc::pull()` with an empty transfer before reporting
            // the error so it does not deadlock.
            send_empty_pull_response(vm_stdin_tx, &pull_header, counters).await;
            send_networking_error(vm_stdin_tx, response_context, counters).await;
            return;
        },
    };

    // Run the blocking `recv()` on its own thread so a waiting socket does not stall the I/O
    // handler loop.
    let vm_stdin_tx: mpsc::Sender<IkcFrame> = vm_stdin_tx.clone();
    let counters: MessageCounters = counters.clone();
    let handle = tokio::task::spawn_blocking(move || {
        let (response, data): (Message, Vec<u8>) = network_daemon.handle_recv(source, syscall_msg);

        // The payload is bounded by the scatter/gather bulk limit. Guard the conversion anyway,
        // releasing the guest's pull with an empty transfer and an error on the unexpected overflow
        // so it does not deadlock.
        let actual_len: u32 = match u32::try_from(data.len()) {
            Ok(n) => n,
            Err(_) => {
                error!("standalone io_handler: recv size overflows u32 (len={})", data.len());
                let empty_header: DataChunkHeader = DataChunkHeader::new(
                    pull_header.source_pid(),
                    pull_header.source_tid(),
                    pull_header.destination_pid(),
                    pull_header.destination_tid(),
                    pull_header.data_addr(),
                    0,
                );
                counters.increment_io_handler_messages_sent();
                counters.increment_io_handler_messages_sent();
                if let Err(error) = vm_stdin_tx
                    .blocking_send(IkcFrame::Bulk(DataChunk::new(empty_header, Vec::new())))
                {
                    error!(
                        "standalone io_handler: failed to send empty bulk response (VM input \
                         channel closed): {error}"
                    );
                }
                let err: Message = response_context.prepare(Message::new(
                    MessageSender::NETWORKD,
                    MessageReceiver::KERNEL,
                    MessageType::Ikc,
                    Some(ErrorCode::InvalidMessage),
                    [0u8; Message::PAYLOAD_SIZE],
                ));
                if let Err(error) = vm_stdin_tx.blocking_send(IkcFrame::Message(err)) {
                    error!(
                        "standalone io_handler: failed to send recv error response (VM input \
                         channel closed): {error}"
                    );
                }
                return;
            },
        };
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
        counters.increment_io_handler_messages_sent();
        counters.increment_io_handler_messages_sent();
        if vm_stdin_tx
            .blocking_send(IkcFrame::Bulk(response_bulk))
            .is_err()
        {
            error!(
                "standalone io_handler: failed to send recv bulk response (VM input channel \
                 closed)"
            );
            return;
        }
        if vm_stdin_tx
            .blocking_send(IkcFrame::Message(response))
            .is_err()
        {
            error!("standalone io_handler: failed to send recv response (VM input channel closed)");
        }
    });

    tokio::spawn(async move {
        if let Err(e) = handle.await {
            error!("standalone io_handler: recv task panicked: {e}");
        }
    });
}

///
/// # Description
///
/// Sends a correlated networking error response, releasing a guest blocked in
/// `ipc::recv()` after a sendto/recvfrom/recv request when the operation cannot proceed.
///
async fn send_networking_error(
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    response_context: ConsoleResponseContext,
    counters: &MessageCounters,
) {
    let error_response: Message = response_context.prepare(Message::new(
        MessageSender::NETWORKD,
        MessageReceiver::KERNEL,
        MessageType::Ikc,
        Some(ErrorCode::OperationNotSupported),
        [0u8; Message::PAYLOAD_SIZE],
    ));
    counters.increment_io_handler_messages_sent();
    if vm_stdin_tx
        .send(IkcFrame::Message(error_response))
        .await
        .is_err()
    {
        error!(
            "standalone io_handler: failed to send networking error response (VM input channel \
             closed)"
        );
    }
}

///
/// # Description
///
/// Pushes an empty bulk transfer back to the guest to release a thread blocked in `ipc::pull()`
/// when a pull-based networking request cannot be served.
///
async fn send_empty_pull_response(
    vm_stdin_tx: &mpsc::Sender<IkcFrame>,
    pull_header: &DataChunkHeader,
    counters: &MessageCounters,
) {
    let header: DataChunkHeader = DataChunkHeader::new(
        pull_header.source_pid(),
        pull_header.source_tid(),
        pull_header.destination_pid(),
        pull_header.destination_tid(),
        pull_header.data_addr(),
        0,
    );
    let bulk: DataChunk = DataChunk::new(header, Vec::new());
    counters.increment_io_handler_messages_sent();
    if vm_stdin_tx.send(IkcFrame::Bulk(bulk)).await.is_err() {
        error!(
            "standalone io_handler: failed to send empty pull response (VM input channel closed)"
        );
    }
}
