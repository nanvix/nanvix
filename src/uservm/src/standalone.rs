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
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::std::collections::VecDeque;
use ::sys::{
    ipc::{
        DataChunk,
        DataChunkHeader,
        IkcFrame,
        Message,
    },
    pm::ThreadIdentifier,
};
use ::syscall::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
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
    pub fn spawn(
        kernel_filename: String,
        initrd_filename: Option<String>,
        initrd_args: Option<String>,
        ramfs_filename: Option<String>,
        stderr: Option<String>,
        snapshot_path: Option<String>,
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
/// Increments the guest credit counter so that the kernel's `get_credits()` sees the pending
/// response and issues a `VmbusRead` to consume it.
///
#[cfg(feature = "hyperlight")]
async fn add_standalone_credit() {
    if let Some((guest_arc, vmem_arc)) = crate::STANDALONE_CREDIT_HANDLES.get() {
        let mut guest = guest_arc.lock().await;
        let mut vmem = vmem_arc.lock().await;
        let _ = guest.add_credit(&mut vmem);
    }
}

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
) {
    let mut input_buffer: VecDeque<u8> = VecDeque::new();
    let mut input_closed: bool = false;

    while let Some(frame) = vm_stdout_rx.recv().await {
        match frame {
            IkcFrame::Message(msg) => {
                let ldm: LinuxDaemonMessage = match LinuxDaemonMessage::try_from_bytes(msg.payload)
                {
                    Ok(ldm) => ldm,
                    Err(e) => {
                        warn!("standalone io_handler: failed to parse message: {e:?}");
                        continue;
                    },
                };

                match ldm.header {
                    LinuxDaemonMessageHeader::WriteRequest => {
                        let tid: ThreadIdentifier = extract_tid(msg.source);
                        let req: WriteRequest = WriteRequest::from_bytes(ldm.payload);
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
                    LinuxDaemonMessageHeader::ReadRequest => {
                        let tid: ThreadIdentifier = extract_tid(msg.source);
                        let req: ReadRequest = ReadRequest::from_bytes(ldm.payload);
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
                    header => {
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
            let response: Message = WriteResponse::build(tid, 0);
            counters.increment_io_thread_messages_received();
            #[cfg(feature = "hyperlight")]
            add_standalone_credit().await;
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
        let response: Message = WriteResponse::build(tid, -1);
        counters.increment_io_thread_messages_received();
        #[cfg(feature = "hyperlight")]
        add_standalone_credit().await;
        if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
            error!("standalone io_handler: failed to send WriteResponse (VM input channel closed)");
        }
        return;
    }

    let written: i32 = match i32::try_from(data.len()) {
        Ok(n) => n,
        Err(_) => {
            error!("standalone io_handler: write size overflows i32 (len={})", data.len());
            let response: Message = WriteResponse::build(tid, -1);
            counters.increment_io_thread_messages_received();
            #[cfg(feature = "hyperlight")]
            add_standalone_credit().await;
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
        let response: Message = WriteResponse::build(tid, -1);
        counters.increment_io_thread_messages_received();
        #[cfg(feature = "hyperlight")]
        add_standalone_credit().await;
        if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
            error!("standalone io_handler: failed to send WriteResponse (VM input channel closed)");
        }
        return;
    }

    // Send WriteResponse back to guest.
    let response: Message = WriteResponse::build(tid, written);
    trace!("standalone io_handler: sending WriteResponse (written={written}, tid={tid:?})");
    counters.increment_io_thread_messages_received();
    #[cfg(feature = "hyperlight")]
    add_standalone_credit().await;
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
            let response: Message = ReadResponse::eof(tid);
            counters.increment_io_thread_messages_received();
            #[cfg(feature = "hyperlight")]
            add_standalone_credit().await;
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
        #[cfg(feature = "hyperlight")]
        add_standalone_credit().await;
        if vm_stdin_tx.send(IkcFrame::Bulk(error_bulk)).await.is_err() {
            error!("standalone io_handler: failed to send bulk response (VM input channel closed)");
            return;
        }
        let empty_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
        let response: Message = ReadResponse::build(tid, -1, empty_buf);
        #[cfg(feature = "hyperlight")]
        add_standalone_credit().await;
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
            let response: Message = ReadResponse::build(tid, -1, empty_buf);
            counters.increment_io_thread_messages_received();
            #[cfg(feature = "hyperlight")]
            add_standalone_credit().await;
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
    #[cfg(feature = "hyperlight")]
    add_standalone_credit().await;
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
    let response: Message = ReadResponse::build(tid, actual_len.cast_signed(), empty_buf);
    #[cfg(feature = "hyperlight")]
    add_standalone_credit().await;
    if vm_stdin_tx.send(IkcFrame::Message(response)).await.is_err() {
        error!("standalone io_handler: failed to send ReadResponse (VM input channel closed)");
    }
}
