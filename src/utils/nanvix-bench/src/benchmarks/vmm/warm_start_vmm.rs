// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::super::{
    CHANNEL_CAPACITY,
    CLEANUP_SLEEP_DURATION,
    DEFAULT_PAYLOAD_SIZE,
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
        unistd::message::{
            ReadRequest,
            ReadResponse,
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
    mem,
    time::Instant,
};
use ::tokio::{
    sync::mpsc,
    time::sleep,
};

impl Benchmark {
    /// In this micro-benchmark we measure the time for a message to travel
    /// all the way from the VMM to the guest application and back. To achieve
    /// this, we connect the user VM to a gateway that emulates linuxd.
    pub async fn run_warm_start_vmm(&mut self) -> Result<()> {
        // Display a progress bar.
        let pb = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        // Payload we are sending over the wire.
        let data = [7u8; DEFAULT_PAYLOAD_SIZE];
        let mut payload: Vec<u8> = Vec::with_capacity(mem::size_of::<u32>() + data.len());
        payload.extend_from_slice(&(DEFAULT_PAYLOAD_SIZE as u32).to_le_bytes());
        payload.extend_from_slice(&data);
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
            // Step 1: Receive the ReadRequest IKC message from the guest.
            let warmup_read_msg: Message = match vcpu_thread_stdout_rx.recv().await {
                Some(IkcFrame::Message(message)) => message,
                Some(IkcFrame::Bulk(_)) => {
                    anyhow::bail!("warmup: unexpected bulk during ReadRequest")
                },
                None => anyhow::bail!("warmup: channel closed during ReadRequest"),
            };
            let warmup_syscall_msg: SystemCallMessage =
                SystemCallMessage::try_from_bytes(warmup_read_msg.payload)
                    .map_err(|_| anyhow::anyhow!("warmup: error parsing SystemCall message"))?;
            let warmup_tid: ThreadIdentifier = { warmup_read_msg.source }.tid;
            if warmup_tid.is_none() {
                anyhow::bail!("warmup: message source has no thread id");
            }
            let _warmup_read_req: ReadRequest = ReadRequest::from_bytes(warmup_syscall_msg.payload);

            // Step 2: Receive the bulk pull request from the guest kernel.
            let warmup_pull_header: DataChunkHeader = match vcpu_thread_stdout_rx.recv().await {
                Some(IkcFrame::Bulk(bulk)) => *bulk.header(),
                Some(IkcFrame::Message(_)) => {
                    anyhow::bail!("warmup: unexpected message during bulk pull")
                },
                None => anyhow::bail!("warmup: channel closed during bulk pull"),
            };

            // Step 3: Send the bulk data response back to the kernel buffer.
            let warmup_bulk_response: DataChunk = DataChunk::new(
                DataChunkHeader::new(
                    warmup_pull_header.source_pid(),
                    warmup_pull_header.source_tid(),
                    warmup_pull_header.destination_pid(),
                    warmup_pull_header.destination_tid(),
                    warmup_pull_header.data_addr(),
                    payload.len() as u32,
                ),
                payload.clone(),
            );
            io_thread_data_tx
                .send(IkcFrame::Bulk(warmup_bulk_response))
                .await?;

            // Step 4: Send ReadResponse metadata.
            let warmup_empty_buf: [u8; ReadResponse::BUFFER_SIZE] =
                [0u8; ReadResponse::BUFFER_SIZE];
            let warmup_read_response: Message = ReadResponse::build(
                warmup_tid,
                payload.len() as i32,
                warmup_empty_buf,
                ProcessIdentifier::KERNEL,
                MessageType::Ikc,
            );
            io_thread_data_tx
                .send(IkcFrame::Message(warmup_read_response))
                .await?;

            // Step 5: Receive the WriteRequest IKC message from the guest.
            match vcpu_thread_stdout_rx.recv().await {
                Some(IkcFrame::Message(_)) => {},
                Some(IkcFrame::Bulk(_)) => {
                    anyhow::bail!("warmup: unexpected bulk during WriteRequest")
                },
                None => anyhow::bail!("warmup: channel closed during WriteRequest"),
            };

            // Step 6: Receive the bulk push data from the guest.
            match vcpu_thread_stdout_rx.recv().await {
                Some(IkcFrame::Bulk(_)) => {},
                Some(IkcFrame::Message(_)) => {
                    anyhow::bail!("warmup: unexpected message during bulk push")
                },
                None => anyhow::bail!("warmup: channel closed during bulk push"),
            };

            // Step 7: Send WriteResponse to acknowledge the write.
            let warmup_write_response: Message = WriteResponse::build(
                warmup_tid,
                payload.len() as i32,
                ProcessIdentifier::KERNEL,
                MessageType::Ikc,
            );
            io_thread_data_tx
                .send(IkcFrame::Message(warmup_write_response))
                .await?;

            sleep(std::time::Duration::from_millis(WARMUP_SLEEP_DURATION)).await;
        }

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        for _ in 0..self.iterations {
            // Step 1: Receive the ReadRequest IKC message from the guest.
            let ipc_read_message: Message = match vcpu_thread_stdout_rx.recv().await {
                Some(IkcFrame::Message(message)) => message,
                Some(IkcFrame::Bulk(_)) => {
                    let reason: String = "unexpected data chunk transfer received while waiting \
                                          for ReadRequest"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
                None => {
                    let reason: String = "user VM channel closed unexpectedly while waiting for \
                                          ReadRequest"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
            };
            let syscall_message: SystemCallMessage =
                match SystemCallMessage::try_from_bytes(ipc_read_message.payload) {
                    Ok(message) => message,
                    Err(_) => {
                        return Err(anyhow::anyhow!(
                            "Error parsing IPC message to SystemCall message"
                        ));
                    },
                };
            let tid: ThreadIdentifier = { ipc_read_message.source }.tid;
            if tid.is_none() {
                return Err(anyhow::anyhow!("unexpected message source: no thread id"));
            }
            let _read_request: ReadRequest = ReadRequest::from_bytes(syscall_message.payload);

            // Step 2: Receive the bulk pull request from the guest kernel.
            let pull_header: DataChunkHeader = match vcpu_thread_stdout_rx.recv().await {
                Some(IkcFrame::Bulk(bulk)) => *bulk.header(),
                Some(IkcFrame::Message(_)) => {
                    let reason: String = "unexpected IKC message received while waiting for bulk \
                                          pull request"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
                None => {
                    let reason: String = "user VM channel closed unexpectedly while waiting for \
                                          bulk pull request"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
            };

            // Now we are ready to push bulk data and ReadResponse, and wait for a WriteRequest.
            let start = Instant::now();

            // Step 3: Send the bulk data response back to the kernel buffer.
            let bulk_response: DataChunk = DataChunk::new(
                DataChunkHeader::new(
                    pull_header.source_pid(),
                    pull_header.source_tid(),
                    pull_header.destination_pid(),
                    pull_header.destination_tid(),
                    pull_header.data_addr(),
                    payload.len() as u32,
                ),
                payload.clone(),
            );
            io_thread_data_tx
                .send(IkcFrame::Bulk(bulk_response))
                .await?;

            // Step 4: Send ReadResponse metadata (buffer is empty; data was sent via bulk).
            let empty_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
            let read_response: Message = ReadResponse::build(
                tid,
                payload.len() as i32,
                empty_buf,
                ProcessIdentifier::KERNEL,
                MessageType::Ikc,
            );
            io_thread_data_tx
                .send(IkcFrame::Message(read_response))
                .await?;

            // Step 5: Receive the WriteRequest IKC message from the guest.
            let _write_request: Message = match vcpu_thread_stdout_rx.recv().await {
                Some(IkcFrame::Message(message)) => message,
                Some(IkcFrame::Bulk(_)) => {
                    let reason: String = "unexpected data chunk transfer received while waiting \
                                          for WriteRequest"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
                None => {
                    let reason: String = "user VM channel closed unexpectedly while waiting for \
                                          WriteRequest"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
            };

            // Step 6: Receive the bulk push data from the guest.
            let _push_data: DataChunk = match vcpu_thread_stdout_rx.recv().await {
                Some(IkcFrame::Bulk(bulk)) => bulk,
                Some(IkcFrame::Message(_)) => {
                    let reason: String = "unexpected IKC message received while waiting for bulk \
                                          push data"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
                None => {
                    let reason: String = "user VM channel closed unexpectedly while waiting for \
                                          bulk push data"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
            };

            latencies.push(start.elapsed().as_micros());

            // Step 7: Send WriteResponse to acknowledge the write.
            let write_response: Message = WriteResponse::build(
                tid,
                payload.len() as i32,
                ProcessIdentifier::KERNEL,
                MessageType::Ikc,
            );
            io_thread_data_tx
                .send(IkcFrame::Message(write_response))
                .await?;

            sleep(std::time::Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;

            pb.inc(1);
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
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        Ok(())
    }
}
