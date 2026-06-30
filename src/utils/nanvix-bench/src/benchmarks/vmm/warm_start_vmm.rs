// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::super::{
    CHANNEL_CAPACITY,
    CLEANUP_SLEEP_DURATION,
    WARMUP_SLEEP_DURATION,
};
use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::log::{
    debug,
    error,
    warn,
};
use ::nanvix::{
    sys::{
        ipc::{
            DataChunk,
            DataChunkHeader,
            IkcFrame,
            Message,
            MessageType,
        },
        pm::{
            ProcessIdentifier,
            ThreadIdentifier,
        },
    },
    syscall::{
        SystemCallMessage,
        SystemCallMessageHeader,
        unistd::message::{
            ReadRequest,
            ReadResponse,
            WriteRequest,
            WriteResponse,
        },
    },
    uservm::{
        UserVm,
        UserVmArgs,
        counters::MessageCounters,
        orchestrator::{
            IoControlCommand,
            IoControlResponse,
        },
    },
};
use ::std::{
    collections::HashMap,
    mem,
    time::Instant,
};
use ::tokio::{
    sync::mpsc,
    time::sleep,
};

enum GuestRequest {
    ReadPull(ThreadIdentifier, DataChunkHeader),
    WritePush(ThreadIdentifier, DataChunk),
}

const MESSAGE_SIZES: [(&str, usize); 7] = [
    ("32B", 32),
    ("1KiB", 1024),
    ("4KiB", 4 * 1024),
    ("8KiB", 8 * 1024),
    ("16KiB", 16 * 1024),
    ("32KiB", 32 * 1024),
    ("64KiB", 64 * 1024),
];
const PAYLOAD_SIZE_PREFIX_BYTES: usize = mem::size_of::<u32>();

fn format_message_size(total_size: usize) -> String {
    if total_size >= 1024 && total_size.is_multiple_of(1024) {
        format!("{}KiB", total_size / 1024)
    } else {
        format!("{total_size}B")
    }
}

fn make_payload(total_size: usize) -> Result<Vec<u8>> {
    let data_size: usize = total_size
        .checked_sub(PAYLOAD_SIZE_PREFIX_BYTES)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "payload size must be at least {PAYLOAD_SIZE_PREFIX_BYTES} bytes because it \
                 includes the length prefix"
            )
        })?;
    let payload_size_header: u32 =
        u32::try_from(data_size).map_err(|e| anyhow::anyhow!("payload size exceeds u32: {e}"))?;
    let mut payload: Vec<u8> = Vec::with_capacity(total_size);
    payload.extend_from_slice(&payload_size_header.to_le_bytes());
    payload.resize(total_size, 7u8);

    Ok(payload)
}

fn message_payloads(payload_size_override: Option<usize>) -> Result<Vec<(String, Vec<u8>)>> {
    let message_sizes: Vec<(String, usize)> = match payload_size_override {
        Some(payload_size) => vec![(format_message_size(payload_size), payload_size)],
        None => MESSAGE_SIZES
            .iter()
            .map(|(label, payload_size)| ((*label).to_string(), *payload_size))
            .collect(),
    };

    message_sizes
        .iter()
        .map(|(label, payload_size)| Ok((label.clone(), make_payload(*payload_size)?)))
        .collect()
}

async fn receive_message(
    output_rx: &mut mpsc::Receiver<IkcFrame>,
    expected: &str,
    context: &str,
) -> Result<Message> {
    match output_rx.recv().await {
        Some(IkcFrame::Message(message)) => Ok(message),
        Some(IkcFrame::Bulk(_)) => {
            anyhow::bail!("{context}: unexpected bulk transfer while waiting for {expected}")
        },
        None => anyhow::bail!("{context}: channel closed while waiting for {expected}"),
    }
}

async fn receive_bulk(
    output_rx: &mut mpsc::Receiver<IkcFrame>,
    expected: &str,
    context: &str,
) -> Result<DataChunk> {
    match output_rx.recv().await {
        Some(IkcFrame::Bulk(bulk)) => Ok(bulk),
        Some(IkcFrame::Message(_)) => {
            anyhow::bail!("{context}: unexpected IKC message while waiting for {expected}")
        },
        None => anyhow::bail!("{context}: channel closed while waiting for {expected}"),
    }
}

fn message_source_tid(message: &Message, context: &str) -> Result<ThreadIdentifier> {
    let tid: ThreadIdentifier = { message.source }.tid;
    if tid == ThreadIdentifier::NONE {
        anyhow::bail!("{context}: message source has no thread id");
    } else {
        Ok(tid)
    }
}

fn chunk_response_header(
    request_header: &DataChunkHeader,
    chunk_len: usize,
) -> Result<DataChunkHeader> {
    Ok(DataChunkHeader::new(
        request_header.source_pid(),
        request_header.source_tid(),
        request_header.destination_pid(),
        request_header.destination_tid(),
        request_header.data_addr(),
        u32::try_from(chunk_len)
            .map_err(|e| anyhow::anyhow!("bulk chunk length exceeds u32: {e}"))?,
    ))
}

async fn send_read_chunk(
    input_tx: &mpsc::Sender<IkcFrame>,
    tid: ThreadIdentifier,
    pull_header: &DataChunkHeader,
    chunk: &[u8],
    context: &str,
) -> Result<()> {
    let bulk_response: DataChunk =
        DataChunk::new(chunk_response_header(pull_header, chunk.len())?, chunk.to_vec());
    input_tx.send(IkcFrame::Bulk(bulk_response)).await?;

    let empty_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
    let read_response: Message = ReadResponse::build(
        tid,
        i32::try_from(chunk.len())
            .map_err(|e| anyhow::anyhow!("{context}: read response length exceeds i32: {e}"))?,
        empty_buf,
        ProcessIdentifier::KERNEL,
        MessageType::Ikc,
    );
    input_tx.send(IkcFrame::Message(read_response)).await?;

    Ok(())
}

async fn receive_guest_request(
    output_rx: &mut mpsc::Receiver<IkcFrame>,
    context: &str,
) -> Result<GuestRequest> {
    let message: Message =
        receive_message(output_rx, "ReadRequest or WriteRequest", context).await?;
    let syscall_message: SystemCallMessage = SystemCallMessage::try_from_bytes(message.payload)
        .map_err(|_| anyhow::anyhow!("{context}: error parsing SystemCall message"))?;
    match syscall_message.header {
        SystemCallMessageHeader::ReadRequest => {
            let tid: ThreadIdentifier = message_source_tid(&message, context)?;
            let read_request: ReadRequest = ReadRequest::from_bytes(syscall_message.payload);
            let requested_count: usize = read_request.count as usize;

            let pull: DataChunk = receive_bulk(output_rx, "bulk pull request", context).await?;
            let pull_header: DataChunkHeader = *pull.header();
            let pull_len: usize = pull_header.data_len() as usize;
            if pull_len > requested_count {
                anyhow::bail!(
                    "{context}: pull length exceeds read request count (pull={pull_len}, \
                     request={requested_count})"
                );
            }

            Ok(GuestRequest::ReadPull(tid, pull_header))
        },
        SystemCallMessageHeader::WriteRequest => {
            let tid: ThreadIdentifier = message_source_tid(&message, context)?;
            let write_request: WriteRequest = WriteRequest::from_bytes(syscall_message.payload);
            let requested_count: usize = write_request.count as usize;

            let push: DataChunk = receive_bulk(output_rx, "bulk push data", context).await?;
            let pushed_len: usize = push.data().len();
            let header_len: usize = push.header().data_len() as usize;
            if pushed_len != header_len {
                anyhow::bail!(
                    "{context}: pushed data length does not match header (data={pushed_len}, \
                     header={header_len})"
                );
            }
            if pushed_len > requested_count {
                anyhow::bail!(
                    "{context}: pushed data length exceeds write request count \
                     (data={pushed_len}, request={requested_count})"
                );
            }

            Ok(GuestRequest::WritePush(tid, push))
        },
        header => anyhow::bail!("{context}: unexpected syscall message: {header:?}"),
    }
}

async fn send_write_response(
    input_tx: &mpsc::Sender<IkcFrame>,
    tid: ThreadIdentifier,
    count: usize,
    context: &str,
) -> Result<()> {
    let write_response: Message = WriteResponse::build(
        tid,
        i32::try_from(count)
            .map_err(|e| anyhow::anyhow!("{context}: write response length exceeds i32: {e}"))?,
        ProcessIdentifier::KERNEL,
        MessageType::Ikc,
    );
    input_tx.send(IkcFrame::Message(write_response)).await?;

    Ok(())
}

async fn run_echo_cycle(
    output_rx: &mut mpsc::Receiver<IkcFrame>,
    input_tx: &mpsc::Sender<IkcFrame>,
    first_request: GuestRequest,
    expected: &[u8],
    context: &str,
) -> Result<(ThreadIdentifier, usize)> {
    let mut sent: usize = 0;
    let mut received: usize = 0;
    let mut next_request: Option<GuestRequest> = Some(first_request);

    loop {
        let request: GuestRequest = match next_request.take() {
            Some(request) => request,
            None => receive_guest_request(output_rx, context).await?,
        };

        match request {
            GuestRequest::ReadPull(tid, pull_header) => {
                if sent == expected.len() {
                    anyhow::bail!("{context}: guest requested more input after full payload");
                }
                let requested_len: usize = pull_header.data_len() as usize;
                if requested_len == 0 {
                    anyhow::bail!("{context}: zero-length pull request while payload remains");
                }

                let remaining: usize = expected.len() - sent;
                let chunk_len: usize = requested_len.min(remaining);
                send_read_chunk(
                    input_tx,
                    tid,
                    &pull_header,
                    &expected[sent..sent + chunk_len],
                    context,
                )
                .await?;
                sent += chunk_len;
            },
            GuestRequest::WritePush(tid, push) => {
                let chunk: &[u8] = push.data();
                if chunk.is_empty() && received < expected.len() {
                    anyhow::bail!("{context}: zero-length push before full payload was echoed");
                }
                let next_received: usize = received
                    .checked_add(chunk.len())
                    .ok_or_else(|| anyhow::anyhow!("{context}: echoed payload length overflow"))?;
                if next_received > expected.len() {
                    anyhow::bail!(
                        "{context}: echoed payload exceeds expected length (received={}, \
                         chunk={}, expected={})",
                        received,
                        chunk.len(),
                        expected.len()
                    );
                }
                if chunk != &expected[received..next_received] {
                    anyhow::bail!("{context}: echoed payload mismatch at offset {received}");
                }

                received = next_received;
                if received == expected.len() {
                    return Ok((tid, chunk.len()));
                }

                send_write_response(input_tx, tid, chunk.len(), context).await?;
            },
        }
    }
}

impl Benchmark {
    /// In this micro-benchmark we measure the time for a message to travel
    /// all the way from the VMM to the guest application and back. To achieve
    /// this, we connect the user VM to a gateway that emulates linuxd.
    pub async fn run_warm_start_vmm(&mut self) -> Result<()> {
        let payloads: Vec<(String, Vec<u8>)> = message_payloads(self.payload_size_override)?;
        let total_iterations: usize = self
            .iterations
            .checked_mul(payloads.len())
            .ok_or_else(|| anyhow::anyhow!("total iteration count overflows usize"))?;
        let total_iterations: u64 = u64::try_from(total_iterations)
            .map_err(|e| anyhow::anyhow!("iteration count exceeds u64: {e}"))?;
        let pb: ProgressBar = ProgressBar::new(total_iterations);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .map_err(|e| anyhow::anyhow!("error creating progress bar: {e}"))?
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        let (vcpu_thread_stdout_tx, mut vcpu_thread_stdout_rx) =
            mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
        let (io_thread_data_tx, memory_thread_data_rx) =
            mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
        let (io_control_command_tx, io_control_rx) =
            mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
        let (io_control_tx, mut io_control_response_rx) =
            mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);

        let kernel_filename: String = format!("{}/bin/kernel.elf", self.workspace_root.display());
        let program: String = self.flavour.get_program(&self.workspace_root);

        // Create shared counters for tracking message flow across threads.
        let counters: MessageCounters = MessageCounters::new();

        let user_vm_handle = UserVm::spawn(UserVmArgs {
            kernel_filename,
            initrd_filename: Some(program),
            initrd_args: None,
            kernel_args: None,
            ramfs_filename: None,
            stderr: Some(if cfg!(windows) { "NUL" } else { "/dev/null" }.to_string()),
            vcpu_thread_stdout_tx,
            memory_thread_data_rx,
            io_control_rx,
            io_control_tx,
            counters,
            snapshot_path: None,
            #[cfg(feature = "gdb")]
            gdb_port: None,
            #[cfg(feature = "profile-time")]
            perf_timings: ::nanvix::uservm::perf::PerfTimings::new(),
            guest_profile_path: None,
        });

        // Warmup: run one untimed echo cycle through the full IKC protocol to trigger
        // lazy initialization so that timed iterations reflect steady-state latency.
        {
            let payload: &[u8] = payloads
                .first()
                .ok_or_else(|| anyhow::anyhow!("no payload sizes configured for warm-start-vmm"))?
                .1
                .as_slice();
            let first_request: GuestRequest =
                receive_guest_request(&mut vcpu_thread_stdout_rx, "warmup").await?;
            let (tid, count): (ThreadIdentifier, usize) = run_echo_cycle(
                &mut vcpu_thread_stdout_rx,
                &io_thread_data_tx,
                first_request,
                payload,
                "warmup",
            )
            .await?;
            send_write_response(&io_thread_data_tx, tid, count, "warmup").await?;

            sleep(std::time::Duration::from_millis(WARMUP_SLEEP_DURATION)).await;
        }

        let mut latencies: HashMap<String, Vec<u128>> = HashMap::new();
        for (label, payload) in &payloads {
            for _ in 0..self.iterations {
                let first_request: GuestRequest =
                    receive_guest_request(&mut vcpu_thread_stdout_rx, "run_warm_start_vmm()")
                        .await?;
                let start = Instant::now();
                let (tid, count): (ThreadIdentifier, usize) = run_echo_cycle(
                    &mut vcpu_thread_stdout_rx,
                    &io_thread_data_tx,
                    first_request,
                    payload,
                    "run_warm_start_vmm()",
                )
                .await?;
                latencies
                    .entry(label.clone())
                    .or_default()
                    .push(start.elapsed().as_micros());

                send_write_response(&io_thread_data_tx, tid, count, "run_warm_start_vmm()").await?;

                sleep(std::time::Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;

                pb.inc(1);
            }
        }

        io_control_command_tx
            .send(IoControlCommand::Shutdown)
            .await?;
        if let Some(response) = io_control_response_rx.recv().await {
            if response != IoControlResponse::Shutdown {
                let reason: String =
                    format!("unexpected control response received during shutdown: {response:?}");
                error!("run_warm_start_vmm(): {reason}");
                anyhow::bail!(reason);
            }
        } else {
            let reason: String = "I/O control response channel closed before receiving shutdown \
                                  acknowledgment"
                .to_string();
            error!("run_warm_start_vmm(): {reason}");
            anyhow::bail!(reason);
        }

        drop(io_thread_data_tx);
        drop(io_control_command_tx);

        match user_vm_handle.await {
            Ok(Ok(exit_status)) => {
                if exit_status != 0 {
                    let reason: String =
                        format!("error running user VM, exit-status={exit_status}");
                    error!("{reason}");
                    return Err(anyhow::anyhow!(reason));
                }
                debug!("User VM: done running");
            },
            Ok(Err(error)) => {
                error!("error running user VM: {error:?}");
                return Err(error);
            },
            Err(error) => {
                let reason: String = format!("error joining user VM task: {error:?}");
                error!("{reason}");
                return Err(anyhow::anyhow!(reason));
            },
        }

        pb.finish();
        println!("Size:\tp50\tp95\tp99 [us]");
        for (label, _) in payloads.iter() {
            if let Some(latencies) = latencies.get_mut(label) {
                latencies.sort();
                let p50: u128 = latencies[(self.iterations as f32 * 0.5) as usize];
                let p95: u128 = latencies[(self.iterations as f32 * 0.95) as usize];
                let p99: u128 = latencies[(self.iterations as f32 * 0.99) as usize];
                println!("{label}:\t{p50}\t{p95}\t{p99}");
            } else {
                warn!("No latencies recorded for {label}");
            }
        }

        Ok(())
    }
}
