// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! High-level orchestration layer for user virtual machines.
//!
//! This crate wires together the asynchronous runtime, virtual machine monitor (VMM), and
//! supporting worker threads that collectively emulate a user virtual machine. It exposes a public
//! API consumed by the Linux daemon and control-plane components to spawn, supervise, and interact
//! with UserVM instances.

//==================================================================================================
// Lint Configuration
//==================================================================================================

// Lints
// NOTE: `deny` instead of `forbid` so that the WHP module (Windows only) can
// `#[allow]` these lints where the Windows Hypervisor Platform API requires casts.
#![deny(clippy::unwrap_used)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
// NOTE: `deny` instead of `forbid` so that test modules can `#[allow(clippy::panic)]`.
#![deny(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]
#![deny(clippy::cast_possible_truncation)]
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Feature Gates
//==================================================================================================

#[cfg(all(feature = "gdb", not(feature = "microvm")))]
compile_error!("feature `gdb` requires feature `microvm`");

//==================================================================================================
// Public Modules
//==================================================================================================

pub mod args;
pub mod counters;
/// Library module for manipulating ELF binaries.
pub mod elf;
/// Host-side guest flamegraph profiler (stack sampling and folded-stack output).
#[cfg(feature = "whp")]
pub mod guest_profiler;
#[cfg(target_os = "linux")]
pub mod io_thread;
pub mod memory_thread;
pub mod orchestrator;
pub mod pal;
#[cfg(feature = "profile-time")]
pub mod perf;
pub mod standalone;
pub mod vmm;

//==================================================================================================
// Private Modules
//==================================================================================================

#[cfg(feature = "hyperlight")]
mod handles;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "hyperlight")]
use crate::handles::UserVmHandles;
#[cfg(feature = "profile-time")]
use crate::perf::PerfTimings;
use crate::{
    counters::MessageCounters,
    memory_thread::{
        AddCreditFn,
        MemoryThread,
    },
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
        LoadSnapshotFn,
        MemoryControlCommand,
        MemoryControlResponse,
        Orchestrator,
        PauseFn,
        ResumeFn,
        ShutdownVcpuFn,
        VcpuControlCommand,
        VcpuControlResponse,
    },
    vmm::{
        MicroVmArgs,
        StdinFn,
        StdoutFn,
        VirtualMemory,
        Vmm,
        guest::Guest,
    },
};
use ::anyhow::Result;
use ::log::{
    error,
    trace,
};
#[cfg(feature = "profile-time")]
use ::std::time::Instant;
use ::std::{
    fs::File,
    io::Write,
    sync::Arc,
    time::Duration,
};
use ::sys::ipc::{
    DataChunk,
    DataChunkHeader,
    IkcFrame,
    Message,
    MessageReceiver,
    MessageSender,
    MessageType,
};
#[cfg(feature = "hyperlight")]
use ::sys::pm::{
    ProcessIdentifier,
    ThreadIdentifier,
};
use ::tokio::{
    sync::{
        Mutex,
        MutexGuard,
        mpsc,
        mpsc::{
            Receiver,
            Sender,
        },
    },
    task::JoinHandle,
};

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Timeout for connecting to System VM.
///
pub const SYSTEM_VM_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// # Description
///
/// Timeout for connecting to control-plane.
///
pub const CONTROL_PLANE_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

///
/// # Description
///
/// Maximum number messages that can be queued in a channel.
///
pub const CHANNEL_CAPACITY: usize = 1024;

//==================================================================================================
// UserVmArgs
//==================================================================================================

/// Bundles all resources required to spawn a [`UserVm`] instance.
pub struct UserVmArgs {
    /// Absolute or relative path to the guest kernel image to boot.
    pub kernel_filename: String,
    /// Optional path to an initrd payload that should be exposed to the guest.
    pub initrd_filename: Option<String>,
    /// Optional string of arguments forwarded to the initrd payload.
    pub initrd_args: Option<String>,
    /// Optional path to a RAM filesystem image exposed to the guest.
    pub ramfs_filename: Option<String>,
    /// Optional path to a file used to capture the guest's stderr stream.
    pub stderr: Option<String>,
    /// Channel used to forward port-I/O writes (messages and data chunk transfers) from the guest.
    pub vcpu_thread_stdout_tx: Sender<IkcFrame>,
    /// Channel providing transfers emitted by the Linux daemon destined for the guest.
    pub memory_thread_data_rx: Receiver<IkcFrame>,
    /// Channel receiving control commands from the orchestrator's I/O subsystem.
    pub io_control_rx: Receiver<IoControlCommand>,
    /// Channel transmitting responses back to the orchestrator's I/O subsystem.
    pub io_control_tx: Sender<IoControlResponse>,
    /// Shared counters for tracking message flow across threads.
    pub counters: MessageCounters,
    /// Optional snapshot path: when set, restore VM state from this snapshot before running.
    pub snapshot_path: Option<String>,
    /// Optional GDB server port (standalone mode only).
    #[cfg(feature = "gdb")]
    pub gdb_port: Option<u16>,
    /// Performance timings collector for fine-grained startup breakdown.
    #[cfg(feature = "profile-time")]
    pub perf_timings: PerfTimings,
    /// When set, enable guest stack profiling and write folded stacks to this path.
    pub guest_profile_path: Option<String>,
}

//==================================================================================================
// UserVm
//==================================================================================================

/// Asynchronous facade responsible for instantiating and supervising a user virtual machine.
pub struct UserVm;

impl UserVm {
    /// Launches a user virtual machine on a Tokio task and returns a handle to its completion.
    pub fn spawn(args: UserVmArgs) -> JoinHandle<Result<u16>> {
        tokio::spawn(async move { UserVm::run(args).await })
    }

    /// Runs the asynchronous event loop powering a user virtual machine.
    ///
    /// # Description
    ///
    /// Instantiates and runs the virtual machine monitor (VMM) using the provided arguments,
    /// wiring up the orchestrator, memory thread, and I/O plumbing required to communicate with
    /// the system VM.
    ///
    /// # Parameters
    ///
    /// - `args`: Aggregation of configuration values and channels consumed by the VMM.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the exit status of the virtual machine.
    /// Otherwise, it returns an error.
    ///
    async fn run(args: UserVmArgs) -> Result<u16> {
        trace!("spawn()");

        #[cfg(feature = "profile-time")]
        let perf_timings: PerfTimings = args.perf_timings.clone();

        let args_guest_profile_path: Option<String> = args.guest_profile_path.clone();
        #[cfg(not(feature = "whp"))]
        let _ = &args_guest_profile_path; // Suppress unused warning when WHP is disabled.

        #[cfg(feature = "profile-time")]
        let run_start: Instant = Instant::now();

        // Phase: Channel setup.
        #[cfg(feature = "profile-time")]
        let channel_setup_start: Instant = Instant::now();

        let (memory_thread_data_tx, vcpu_thread_stdin_rx): (Sender<IkcFrame>, Receiver<IkcFrame>) =
            mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
        let (memory_control_tx, memory_thread_control_rx): (
            Sender<MemoryControlCommand>,
            Receiver<MemoryControlCommand>,
        ) = mpsc::channel::<MemoryControlCommand>(CHANNEL_CAPACITY);
        let (memory_thread_control_tx, memory_control_rx): (
            Sender<MemoryControlResponse>,
            Receiver<MemoryControlResponse>,
        ) = mpsc::channel::<MemoryControlResponse>(CHANNEL_CAPACITY);
        let (vcpu_control_tx, vcpu_thread_control_rx): (
            Sender<VcpuControlCommand>,
            Receiver<VcpuControlCommand>,
        ) = mpsc::channel::<VcpuControlCommand>(CHANNEL_CAPACITY);
        let (vcpu_thread_control_tx, mut vcpu_control_rx): (
            Sender<VcpuControlResponse>,
            Receiver<VcpuControlResponse>,
        ) = mpsc::channel::<VcpuControlResponse>(CHANNEL_CAPACITY);

        #[cfg(not(feature = "hyperlight"))]
        let vmm_stderr_fn: Box<dyn Write + Send> = match get_stderr_writer(args.stderr.clone()) {
            Ok(vmm_stderr_fn) => vmm_stderr_fn,
            Err(e) => {
                let reason: String = format!(
                    "failed to get stderr writer (args.stderr={:?}, error={e:?})",
                    args.stderr.clone()
                );
                error!("{reason}");
                anyhow::bail!(reason);
            },
        };

        // Move the stdout sender out of args so no extra clone keeps the
        // data_rx channel alive after the VMM thread finishes.
        // Output function used for emulating I/O port writes.
        #[cfg(feature = "hyperlight")]
        let bulk_stdout_tx: Sender<IkcFrame> = args.vcpu_thread_stdout_tx.clone();
        let vmm_stdout_fn: Box<StdoutFn> = output_fn(args.vcpu_thread_stdout_tx);

        // Input function used for emulating I/O port reads.
        #[cfg(not(feature = "hyperlight"))]
        let ikc_pending: std::sync::Arc<std::sync::atomic::AtomicBool> =
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        #[cfg(not(feature = "hyperlight"))]
        let vmm_stdin_fn: Box<StdinFn> =
            build_input_fn(vcpu_thread_stdin_rx, args.counters.clone(), ikc_pending.clone());

        #[cfg(feature = "hyperlight")]
        let handles: UserVmHandles = handles::UserVmHandles::default();

        // Bulk output function for hyperlight VmbusBulkWrite host function.
        #[cfg(feature = "hyperlight")]
        let vmm_bulk_stdout_fn: Box<crate::vmm::BulkStdoutFn> =
            bulk_output_fn(bulk_stdout_tx, handles.clone());

        // Shared buffer for pending bulk read data (VmbusBulkRead host function).
        #[cfg(feature = "hyperlight")]
        let pending_bulk_data: Arc<std::sync::Mutex<Vec<u8>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));

        #[cfg(feature = "hyperlight")]
        let vmm_stdin_fn: Box<StdinFn> = build_input_fn(
            vcpu_thread_stdin_rx,
            args.counters.clone(),
            handles.clone(),
            pending_bulk_data.clone(),
        );

        // Bulk input function for hyperlight VmbusBulkRead host function.
        #[cfg(feature = "hyperlight")]
        let vmm_bulk_stdin_fn: Box<crate::vmm::BulkStdinFn> =
            build_bulk_input_fn(pending_bulk_data.clone());

        #[cfg(feature = "profile-time")]
        perf_timings.set_channel_setup(channel_setup_start.elapsed().as_micros() as u64);

        #[allow(unused_mut)] // `mut` needed only with `whp` for enable_guest_profiler().
        let mut microvm: Vmm = Vmm::new(MicroVmArgs {
            input: vmm_stdin_fn,
            output: vmm_stdout_fn,
            #[cfg(feature = "hyperlight")]
            bulk_output: vmm_bulk_stdout_fn,
            #[cfg(feature = "hyperlight")]
            bulk_input: vmm_bulk_stdin_fn,
            #[cfg(not(feature = "hyperlight"))]
            stderr: vmm_stderr_fn,
            #[cfg(feature = "hyperlight")]
            stderr_path: args.stderr.clone(),
            control_rx: vcpu_thread_control_rx,
            control_tx: vcpu_thread_control_tx,
            kernel_filename: args.kernel_filename,
            initrd_filename: args.initrd_filename.clone(),
            initrd_args: args.initrd_args.clone(),
            ramfs_filename: args.ramfs_filename.clone(),
            restoring_from_snapshot: args.snapshot_path.is_some(),
            #[cfg(all(feature = "microvm", not(feature = "hyperlight")))]
            ikc_pending: ikc_pending.clone(),
            #[cfg(feature = "gdb")]
            gdb_port: args.gdb_port,
            #[cfg(feature = "profile-time")]
            perf_timings: perf_timings.clone(),
        })?;

        // If a snapshot path is provided, restore VM state from the snapshot.
        if let Some(snapshot_path) = args.snapshot_path {
            microvm.load_snapshot(snapshot_path).await?;
        }

        // Enable guest profiler if requested (WHP only).
        #[cfg(feature = "whp")]
        let guest_profiler = if args_guest_profile_path.is_some() {
            Some(microvm.enable_guest_profiler())
        } else {
            None
        };
        #[cfg(not(feature = "whp"))]
        let _guest_profiler: Option<()> = None;

        let vmem: Arc<Mutex<VirtualMemory>> = microvm.vmem();
        let guest: Arc<Mutex<Guest>> = microvm.guest();

        // Set hyperlight handles in counters for this UserVM instance.
        #[cfg(feature = "hyperlight")]
        {
            handles.set_guest_handle(guest.clone()).await;
            handles.set_vmem_handle(vmem.clone()).await;
        }

        // Phase: Thread spawning.
        #[cfg(feature = "profile-time")]
        let thread_spawn_start: Instant = Instant::now();

        // Create a thread that reads from vm_rx and writes to vm_rx2.
        let memory_thread: MemoryThread = MemoryThread::new(
            args.memory_thread_data_rx,
            memory_thread_data_tx,
            memory_thread_control_rx,
            memory_thread_control_tx,
            add_credit_fn(
                guest.clone(),
                vmem.clone(),
                #[cfg(not(feature = "hyperlight"))]
                microvm.ikc_notifier(),
            ),
            args.counters.clone(),
        );
        let memory_thread: JoinHandle<()> = memory_thread.spawn();

        let vmm_thread: JoinHandle<Result<u16>> = Vmm::spawn(microvm.clone());

        // Wait right after spawning the vCPU thread such that we populate the pthread id holder
        // before actually starting the vCPU.
        let vcpu_tid: u64 = match vcpu_control_rx.recv().await {
            Some(VcpuControlResponse::Tid(tid)) => {
                trace!("Received vCPU thread tid: {tid}");
                tid
            },
            _ => {
                let reason: String = "the vCPU thread has disconnected".to_string();
                error!("spawn(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let filename: String = args
            .initrd_filename
            .unwrap_or("bin/default.elf".to_string());
        let orchestrator_thread: Orchestrator = Orchestrator::new(
            vcpu_tid,
            args.io_control_rx,
            args.io_control_tx,
            memory_control_rx,
            memory_control_tx,
            vcpu_control_rx,
            vcpu_control_tx,
            pause_microvm(guest.clone(), vmem.clone()),
            resume_microvm(guest.clone(), vmem.clone()),
            create_snapshot_fn(microvm.clone(), filename.clone()),
            load_snapshot_fn(microvm.clone(), filename.clone()),
            shutdown_vcpu_fn(microvm.clone()),
        );

        let orchestrator_thread_handle: JoinHandle<Result<()>> = orchestrator_thread.spawn();

        #[cfg(feature = "profile-time")]
        perf_timings.set_thread_spawn(thread_spawn_start.elapsed().as_micros() as u64);

        let exit_code: Result<u16> = match vmm_thread.await {
            Ok(exit_code) => exit_code,
            Err(error) => {
                let reason: String = format!("failed to join vmm thread (error={error:?})");
                error!("spawn(): {reason}");
                anyhow::bail!(reason)
            },
        };

        if let Err(error) = orchestrator_thread_handle.await {
            error!("spawn(): failed to join orchestrator thread (error={error:?})");
            // Don't bail, in order to cleanup the other the other tasks properly.
        }

        if let Err(error) = memory_thread.await {
            error!("spawn(): error joining memory thread (error={error:?})");
            // Don't bail, in order to cleanup the other the other tasks properly.
        }

        #[cfg(feature = "profile-time")]
        perf_timings.set_total(run_start.elapsed().as_micros() as u64);

        // Write guest profiler folded stacks if profiling was enabled.
        #[cfg(feature = "whp")]
        if let (Some(profiler), Some(path)) = (guest_profiler, &args_guest_profile_path) {
            let sample_count = profiler.handle().lock().map(|s| s.len()).unwrap_or(0);
            let mut sym_paths: Vec<std::path::PathBuf> = Vec::new();
            if let Ok(p) = std::env::var("NANVIX_KERNEL_SYMBOLS") {
                sym_paths.push(std::path::PathBuf::from(p));
            }
            if let Ok(p) = std::env::var("NANVIX_USER_SYMBOLS") {
                sym_paths.push(std::path::PathBuf::from(p));
            }
            let sym_refs: Vec<&std::path::Path> = sym_paths.iter().map(|p| p.as_path()).collect();
            let resolver = crate::guest_profiler::SymbolResolver::from_elf_files(&sym_refs);
            if let Err(e) = profiler.write_folded(path, |addr| resolver.resolve(addr)) {
                error!("Failed to write guest profile: {e:?}");
            } else {
                eprintln!("GUEST_PROFILE: wrote {} samples to {}", sample_count, path);
            }
        }

        exit_code
    }
}

fn create_snapshot_fn(microvm: Vmm, filename: String) -> Box<LoadSnapshotFn> {
    Box::new(move || {
        let microvm = microvm.clone();
        let filename = filename.clone();
        Box::pin(async move { microvm.create_snapshot(filename).await })
    })
}

fn load_snapshot_fn(microvm: Vmm, filename: String) -> Box<LoadSnapshotFn> {
    Box::new(move || {
        let microvm = microvm.clone();
        let filename = filename.clone();
        Box::pin(async move { microvm.load_snapshot(filename).await })
    })
}

fn pause_microvm(guest: Arc<Mutex<Guest>>, vmem: Arc<Mutex<VirtualMemory>>) -> Box<PauseFn> {
    Box::new(move || {
        let guest: Arc<Mutex<Guest>> = guest.clone();
        let vmem: Arc<Mutex<VirtualMemory>> = vmem.clone();
        Box::pin(async move {
            let mut guest = guest.lock().await;
            let mut vmem: MutexGuard<VirtualMemory> = vmem.lock().await;
            guest.pause_vm(&mut vmem)
        })
    })
}

fn resume_microvm(guest: Arc<Mutex<Guest>>, vmem: Arc<Mutex<VirtualMemory>>) -> Box<ResumeFn> {
    Box::new(move || {
        let guest: Arc<Mutex<Guest>> = guest.clone();
        let vmem: Arc<Mutex<VirtualMemory>> = vmem.clone();
        Box::pin(async move {
            let mut guest = guest.lock().await;
            let mut vmem: MutexGuard<VirtualMemory> = vmem.lock().await;
            guest.resume_vm(&mut vmem)
        })
    })
}

fn shutdown_vcpu_fn(vmm: Vmm) -> Box<ShutdownVcpuFn> {
    Box::new(move || {
        vmm.request_shutdown();
    })
}

fn add_credit_fn(
    guest: Arc<Mutex<Guest>>,
    vmem: Arc<Mutex<VirtualMemory>>,
    #[cfg(not(feature = "hyperlight"))] notifier: crate::vmm::IkcNotifier,
) -> Box<AddCreditFn> {
    Box::new(move || {
        let guest: Arc<Mutex<Guest>> = guest.clone();
        let vmem: Arc<Mutex<VirtualMemory>> = vmem.clone();
        #[cfg(not(feature = "hyperlight"))]
        let notifier: crate::vmm::IkcNotifier = notifier.clone();
        Box::pin(async move {
            // Scope the locks so they are released before the IRQ injection.
            {
                let mut guest = guest.lock().await;
                let mut vmem = vmem.lock().await;
                guest.add_credit(&mut vmem)?;
            }
            // Inject an edge-triggered IRQ to wake the guest from HLT
            // immediately, rather than waiting for the next PIT timer tick.
            // This is lock-free — the notifier uses a duplicated VM fd.
            #[cfg(not(feature = "hyperlight"))]
            notifier.notify()?;
            Ok(())
        })
    })
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Obtains a buffered writer for the virtual machine's standard error device.
///
/// When a file path is provided the writer targets the specified file, creating or truncating it
/// as needed. Otherwise the host process' standard error stream is used.
///
/// # Parameters
///
/// * `vm_stderr` - Optional file path used to capture the guest's stderr output.
///
/// # Returns
///
/// On success, the function returns a buffered writer for the virtual machine's standard error
/// stream. An error is returned when the target file cannot be created.
///
pub fn get_stderr_writer(vm_stderr: Option<String>) -> Result<Box<dyn Write + Send>> {
    // Obtain a buffered writer for the virtual machine's standard error device.
    let file_writer: Box<dyn Write + Send> = if let Some(vm_stderr) = vm_stderr {
        // Standard error was set to a file. Attempt to open file and create a writer.
        let file = File::options()
            .read(false)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&vm_stderr)?;
        Box::new(file)
    } else {
        // Standard error was not set to a file. Fallback to stderr.
        Box::new(std::io::stderr())
    };
    Ok(file_writer)
}

///
/// # Description
///
/// Builds an input callback that delivers messages from the Linux daemon to the virtual machine.
///
/// # Parameters
///
/// - `input_queue` - Channel used to receive emulated port-I/O requests originating from the
///   Linux daemon.
/// - `counters` - Shared counters for tracking message flow across threads.
/// - `handles` - Shared handles for guest and virtual memory manager (hyperlight only).
///
/// # Returns
///
/// A boxed closure compatible with the VMM's stdin handler implementation.
///
#[cfg(not(feature = "hyperlight"))]
pub fn build_input_fn(
    mut input_queue: Receiver<IkcFrame>,
    counters: MessageCounters,
    ikc_pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Box<StdinFn> {
    // Input function used for emulating I/O port reads.
    let input = move |guest: &Arc<Mutex<Guest>>,
                      vmem: &Arc<Mutex<VirtualMemory>>,
                      data,
                      size|
          -> Result<()> {
        use std::mem;
        on_input_function_called(&counters);

        // Check for invalid operand size.
        if size != mem::size_of::<u32>() {
            let reason: String = format!("invalid operand size (size={size:?})");
            error!("input(): {reason}");
            anyhow::bail!(reason);
        }

        match input_queue.blocking_recv() {
            Some(transfer) => {
                match transfer {
                    IkcFrame::Message(mut msg) => {
                        // Label: uservm::lib::vm_input::vm_exit()
                        profiler::timestamp_message!(
                            &mut msg.payload,
                            mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                        );

                        on_message_received_from_memory_thread(&counters);
                        msg.message_type = MessageType::Ikc;
                        let mut locked_guest: MutexGuard<'_, Guest> = guest.blocking_lock();
                        let mut locked_vm: MutexGuard<'_, VirtualMemory> = vmem.blocking_lock();

                        // Label: uservm::lib::vm_input::vm_write_bytes()
                        profiler::timestamp_message!(
                            &mut msg.payload,
                            mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                        );

                        locked_vm.write_bytes(data as u64, &msg.to_bytes())?;
                        locked_guest.consume_credit(&mut locked_vm)?;
                        ikc_pending.store(false, std::sync::atomic::Ordering::Release);
                    },
                    IkcFrame::Bulk(mut bulk) => {
                        // Write bulk data directly into guest memory at the address specified
                        // by the pull request.
                        //
                        // NOTE: this creates a temporal coupling with the kernel's main
                        // loop. The PullResponse notification message written below must be
                        // consumed by the kernel before the next IKC message arrives on the
                        // same `data` buffer. On a single-core guest this is guaranteed
                        // because the kernel processes messages sequentially, but the
                        // invariant should be preserved if the design evolves.
                        on_message_received_from_memory_thread(&counters);
                        // Label: uservm::lib::vm_input::vmexit()
                        profiler::timestamp_message!(bulk.data_mut(), 0);
                        let mut locked_guest: MutexGuard<'_, Guest> = guest.blocking_lock();
                        let mut locked_vm: MutexGuard<'_, VirtualMemory> = vmem.blocking_lock();

                        let dest_addr: u64 = bulk.header().data_addr() as u64;
                        let actual_len: usize = bulk.data().len();
                        trace!(
                            "input(): writing {actual_len} bulk bytes to guest at {dest_addr:#x}"
                        );
                        // Label: uservm::lib::vm_input::vm_write_bytes()
                        profiler::timestamp_message!(bulk.data_mut(), 0);
                        locked_vm.write_bytes(dest_addr, bulk.data())?;

                        // Construct a PullResponse notification message. The kernel's
                        // main loop reads this from the regular message buffer, detects the
                        // message type, and wakes the sleeping pull thread.
                        let completion_header: DataChunkHeader = DataChunkHeader::new(
                            bulk.header().source_pid(),
                            bulk.header().source_tid(),
                            bulk.header().destination_pid(),
                            bulk.header().destination_tid(),
                            bulk.header().data_addr(),
                            u32::try_from(actual_len).map_err(|e| {
                                let reason: String = format!("bulk data length exceeds u32: {e}");
                                error!("{reason}");
                                anyhow::Error::msg(reason)
                            })?,
                        );
                        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
                        payload[..DataChunkHeader::SIZE]
                            .copy_from_slice(&completion_header.to_bytes());
                        let completion_msg: Message = Message::new(
                            MessageSender::KERNEL,
                            MessageReceiver::KERNEL,
                            MessageType::PullResponse,
                            None,
                            payload,
                        );
                        locked_vm.write_bytes(data as u64, &completion_msg.to_bytes())?;
                        locked_guest.consume_credit(&mut locked_vm)?;
                        ikc_pending.store(false, std::sync::atomic::Ordering::Release);
                    },
                }
            },
            // Channel has disconnected.
            None => {
                let reason: String = "channel has been disconnected".to_string();
                error!("input(): {reason}");
                anyhow::bail!(reason);
            },
        }

        Ok(())
    };

    Box::new(input)
}

#[cfg(feature = "hyperlight")]
pub fn build_input_fn(
    mut input_queue: Receiver<IkcFrame>,
    counters: MessageCounters,
    handles: UserVmHandles,
    pending_bulk_data: Arc<std::sync::Mutex<Vec<u8>>>,
) -> Box<StdinFn> {
    let input = move || -> Result<Vec<u8>, hyperlight_host::HyperlightError> {
        on_input_function_called(&counters);
        match input_queue.blocking_recv() {
            Some(IkcFrame::Message(mut msg)) => {
                // Label: uservm::lib::vm_input::vm_exit()
                profiler::timestamp_message!(
                    &mut msg.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                );

                on_message_received_from_memory_thread(&counters);
                msg.message_type = MessageType::Ikc;

                let guest_arc: Arc<Mutex<Guest>> = handles.get_guest_handle().ok_or_else(|| {
                    let reason: &str = "guest handle not set in UserVmHandles";
                    error!("input(): {reason}");
                    hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason))
                })?;

                let mut locked_guest: MutexGuard<'_, Guest> = guest_arc.blocking_lock();

                let vmem_arc: Arc<Mutex<VirtualMemory>> =
                    handles.get_vmem_handle().ok_or_else(|| {
                        let reason: &str = "vmem handle not set in UserVmHandles";
                        error!("input(): {reason}");
                        hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason))
                    })?;

                let mut locked_vmem: MutexGuard<'_, VirtualMemory> = vmem_arc.blocking_lock();

                // Label: uservm::lib::vm_input::vm_write_bytes()
                profiler::timestamp_message!(
                    &mut msg.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                );

                locked_guest.consume_credit(&mut locked_vmem)?;
                Ok(msg.to_bytes().to_vec())
            },
            Some(IkcFrame::Bulk(mut bulk)) => {
                // Handle data chunk transfer: store the bulk payload in the shared
                // pending_bulk_data buffer and return only the PullResponse notification
                // message (64 bytes). The kernel will then call VmbusBulkRead in a loop
                // to retrieve the bulk data in small chunks that fit in the slab allocator.
                on_message_received_from_memory_thread(&counters);

                // Label: uservm::lib::vm_input::vmexit()
                profiler::timestamp_message!(bulk.data_mut(), 0);

                let guest_arc: Arc<Mutex<Guest>> = handles.get_guest_handle().ok_or_else(|| {
                    let reason: &str = "guest handle not set in UserVmHandles";
                    error!("input(): {reason}");
                    hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason))
                })?;
                let vmem_arc: Arc<Mutex<VirtualMemory>> =
                    handles.get_vmem_handle().ok_or_else(|| {
                        let reason: &str = "vmem handle not set in UserVmHandles";
                        error!("input(): {reason}");
                        hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason))
                    })?;

                let mut locked_guest: MutexGuard<'_, Guest> = guest_arc.blocking_lock();
                let mut locked_vmem: MutexGuard<'_, VirtualMemory> = vmem_arc.blocking_lock();

                let actual_len: usize = bulk.data().len();
                trace!("input(): storing {actual_len} bulk bytes for VmbusBulkRead");

                // Extract header fields before consuming the bulk data.
                let source_pid: ProcessIdentifier = bulk.header().source_pid();
                let source_tid: ThreadIdentifier = bulk.header().source_tid();
                let dest_pid: ProcessIdentifier = bulk.header().destination_pid();
                let dest_tid: ThreadIdentifier = bulk.header().destination_tid();
                let data_addr: u32 = bulk.header().data_addr();

                // Store the bulk data in the shared buffer for VmbusBulkRead to consume.
                {
                    let mut buf = pending_bulk_data.lock().map_err(|e| {
                        let reason: String = format!("failed to lock pending_bulk_data: {e}");
                        error!("input(): {reason}");
                        hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason))
                    })?;
                    *buf = bulk.into_data();
                }

                // Construct a PullResponse notification message (fits in Slab128).
                let actual_len_u32: u32 = u32::try_from(actual_len).map_err(|e| {
                    let reason: String = format!("bulk data length exceeds u32: {e}");
                    error!("input(): {reason}");
                    hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason))
                })?;
                let completion_header: DataChunkHeader = DataChunkHeader::new(
                    source_pid,
                    source_tid,
                    dest_pid,
                    dest_tid,
                    data_addr,
                    actual_len_u32,
                );
                let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
                payload[..DataChunkHeader::SIZE].copy_from_slice(&completion_header.to_bytes());
                let completion_msg: Message = Message::new(
                    MessageSender::KERNEL,
                    MessageReceiver::KERNEL,
                    MessageType::PullResponse,
                    None,
                    payload,
                );

                locked_guest.consume_credit(&mut locked_vmem)?;
                Ok(completion_msg.to_bytes().to_vec())
            },

            // Channel has disconnected.
            None => {
                let reason: String = "channel has been disconnected".to_string();
                error!("input(): {reason}");
                Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)))
            },
        }
    };

    Box::new(input)
}

/// Builds a bulk input callback for the hyperlight `VmbusBulkRead` host function.
///
/// Each call drains up to `MAX_CHUNK` bytes from the shared pending bulk data buffer.
/// Returns an empty `Vec` when all data has been consumed, signalling the kernel to stop.
#[cfg(feature = "hyperlight")]
fn build_bulk_input_fn(
    pending_bulk_data: Arc<std::sync::Mutex<Vec<u8>>>,
) -> Box<crate::vmm::BulkStdinFn> {
    /// Maximum chunk size returned per VmbusBulkRead call. Must stay under the kernel's
    /// 512-byte slab ceiling after FlatBuffer serialization overhead (~100 bytes).
    /// FIXME (#1779): relying on the 512-byte slab tier is fragile — consider targeting 256-byte.
    const MAX_CHUNK: usize = 400;

    let bulk_input = move || -> Result<Vec<u8>, hyperlight_host::HyperlightError> {
        let mut buf = pending_bulk_data.lock().map_err(|e| {
            let reason: String = format!("failed to lock pending_bulk_data: {e}");
            error!("build_bulk_input_fn(): {reason}");
            hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason))
        })?;

        if buf.is_empty() {
            return Ok(Vec::new());
        }

        let chunk_len: usize = buf.len().min(MAX_CHUNK);
        let chunk: Vec<u8> = buf.drain(..chunk_len).collect();
        trace!("VmbusBulkRead: returning {} bytes ({} remaining)", chunk_len, buf.len());
        Ok(chunk)
    };

    Box::new(bulk_input)
}

///
/// # Description
///
/// Builds an output callback that forwards messages and data chunk transfers emitted by the virtual
/// machine. The callback inspects the [`VmBusMessage`] to determine whether the guest is
/// sending a standard IKC message or a data chunk transfer and constructs the appropriate
/// [`Transfer`] variant.
///
/// # Parameters
///
/// - `queue` - Channel used to forward transfers emitted by the virtual machine to the Linux
///   daemon.
///
/// # Returns
///
/// A boxed closure compatible with the VMM's stdout handler implementation.
///
pub fn output_fn(queue: Sender<IkcFrame>) -> Box<StdoutFn> {
    // Output function used for emulating I/O port writes.
    #[cfg(not(feature = "hyperlight"))]
    let output =
        move |vm: &Arc<Mutex<VirtualMemory>>, envelope: &::sys::ipc::VmBusMessage| -> Result<()> {
            use std::mem;

            if envelope.is_ikc() {
                // Standard IKC message: read the message from guest memory.
                let mut bytes: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
                trace!("output(): reading message from user VM");
                vm.blocking_lock()
                    .read_bytes(envelope.message_addr() as u64, &mut bytes)?;

                let mut message: Message = match Message::try_from_bytes(bytes) {
                    Ok(message) => message,
                    Err(err) => {
                        let reason: String = format!("failed to parse message: {err:?}");
                        error!("output(): {reason}");
                        anyhow::bail!(reason);
                    },
                };

                // Label: uservm::lib::vm_output::send()
                profiler::timestamp_message!(
                    &mut message.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                );

                trace!("output(): forwarding message to system VM");
                if let Err(e) = queue.blocking_send(IkcFrame::Message(message)) {
                    let reason: String = format!("failed to send message: {e:?}");
                    error!("output(): {reason}");
                    anyhow::bail!(reason);
                }

                trace!("output(): message forwarded to system VM");
            } else {
                // Data chunk transfer: `message_addr` points to a DataChunkHeader.
                let mut header_bytes: [u8; DataChunkHeader::SIZE] = [0; DataChunkHeader::SIZE];
                vm.blocking_lock()
                    .read_bytes(envelope.message_addr() as u64, &mut header_bytes)?;
                let header: DataChunkHeader = DataChunkHeader::try_from_bytes(header_bytes)
                    .map_err(|e| {
                        let reason: String =
                            format!("failed to parse data chunk transfer header: {e:?}");
                        error!("output(): {reason}");
                        anyhow::anyhow!(reason)
                    })?;

                let data_len: usize = header.data_len() as usize;
                let data_addr: u64 = header.data_addr() as u64;

                // Allocate a buffer and read the bulk payload from guest memory.
                let mut data: Vec<u8> = vec![0u8; data_len];
                trace!("output(): reading {data_len} bytes from guest memory at {data_addr:#x}");
                vm.blocking_lock().read_bytes(data_addr, &mut data)?;

                // Label: uservm::lib::vm_output::send()
                profiler::timestamp_message!(&mut data, 0);

                let bulk: DataChunk = DataChunk::new(header, data);

                trace!("output(): forwarding data chunk transfer ({data_len} bytes)");
                if let Err(e) = queue.blocking_send(IkcFrame::Bulk(bulk)) {
                    let reason: String = format!("failed to send data chunk transfer: {e:?}");
                    error!("output(): {reason}");
                    anyhow::bail!(reason);
                }
            }

            Ok(())
        };

    #[cfg(feature = "hyperlight")]
    let output = move |data: Vec<u8>| -> Result<i32, hyperlight_host::HyperlightError> {
        let bytes: &[u8] = data.as_slice();
        let expected_length: usize = ::core::mem::size_of::<Message>();
        let payload: [u8; ::core::mem::size_of::<Message>()] = match bytes.try_into() {
            Ok(value) => value,
            Err(_) => {
                let reason: String = format!(
                    "failed to convert payload: expected {} bytes, got {}",
                    expected_length,
                    bytes.len()
                );
                error!("output(): {}", reason);
                return Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(
                    reason,
                )));
            },
        };

        let mut message: Message = match Message::try_from_bytes(payload) {
            Ok(message) => message,
            Err(err) => {
                let reason: String = format!("failed to parse message: {:?}", err);
                error!("output(): {}", reason);
                return Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(
                    reason,
                )));
            },
        };

        // Label: uservm::lib::vm_output::send()
        profiler::timestamp_message!(
            &mut message.payload,
            std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
        );

        if let Err(e) = queue.blocking_send(IkcFrame::Message(message)) {
            let reason: String = format!("failed to send message: {:?}", e);
            error!("output(): {}", reason);
            return Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)));
        }

        match i32::try_from(data.len()) {
            Ok(length) => Ok(length),
            Err(_) => {
                let reason: String =
                    format!("failed to convert payload length {} to i32", data.len());
                error!("output(): {}", reason);
                Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)))
            },
        }
    };

    Box::new(output)
}

///
/// # Description
///
/// Builds a bulk output callback for the hyperlight VmbusBulkWrite host function. The kernel
/// sends only the serialized [`DataChunkHeader`] (24 bytes). This callback reads the actual
/// bulk payload from guest shared memory using the GPA stored in the header's `data_addr` field,
/// then constructs a [`IkcFrame::Bulk`] that is forwarded to linuxd via the standard transfer
/// queue.
///
/// # Parameters
///
/// - `queue` - Channel used to forward transfers emitted by the virtual machine to the Linux
///   daemon.
/// - `handles` - Shared handles for guest and virtual memory manager for reading guest memory.
///
/// # Returns
///
/// A boxed closure compatible with the VMM's bulk output handler implementation.
///
#[cfg(feature = "hyperlight")]
pub fn bulk_output_fn(
    queue: Sender<IkcFrame>,
    _handles: UserVmHandles,
) -> Box<crate::vmm::BulkStdoutFn> {
    let output = move |data: Vec<u8>| -> Result<i32, hyperlight_host::HyperlightError> {
        // The kernel sends header + payload combined (via __phys_memcpy).
        if data.len() < DataChunkHeader::SIZE {
            let reason: String = format!(
                "bulk output data too short: expected at least {} bytes, got {}",
                DataChunkHeader::SIZE,
                data.len()
            );
            error!("bulk_output(): {reason}");
            return Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)));
        }

        let mut header_bytes: [u8; DataChunkHeader::SIZE] = [0u8; DataChunkHeader::SIZE];
        header_bytes.copy_from_slice(&data[..DataChunkHeader::SIZE]);
        let header: DataChunkHeader =
            DataChunkHeader::try_from_bytes(header_bytes).map_err(|e| {
                let reason: String = format!("failed to parse data chunk transfer header: {e:?}");
                error!("bulk_output(): {reason}");
                hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason))
            })?;

        // Extract the inline payload that follows the header, validating its length
        // against the header's data_len field.
        let expected_len: usize = header.data_len() as usize;
        let actual_len: usize = data.len() - DataChunkHeader::SIZE;
        if actual_len < expected_len {
            let reason: String = format!(
                "bulk output payload truncated: header expects {} bytes, got {}",
                expected_len, actual_len
            );
            error!("bulk_output(): {reason}");
            return Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)));
        }
        let mut payload_data: Vec<u8> =
            data[DataChunkHeader::SIZE..DataChunkHeader::SIZE + expected_len].to_vec();

        // Label: uservm::lib::vm_output::send()
        profiler::timestamp_message!(&mut payload_data, 0);

        let data_len: usize = payload_data.len();
        let bulk: DataChunk = DataChunk::new(header, payload_data);

        trace!("bulk_output(): forwarding data chunk transfer ({data_len} bytes)");
        if let Err(e) = queue.blocking_send(IkcFrame::Bulk(bulk)) {
            let reason: String = format!("failed to send data chunk transfer: {e:?}");
            error!("bulk_output(): {reason}");
            return Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)));
        }

        // Return the total logical size (header + data) to signal success to the kernel.
        let total_len: usize = DataChunkHeader::SIZE + data_len;
        match i32::try_from(total_len) {
            Ok(length) => Ok(length),
            Err(_) => {
                let reason: String = format!("failed to convert payload length {total_len} to i32");
                error!("bulk_output(): {reason}");
                Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)))
            },
        }
    };

    Box::new(output)
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Handler to be called whenever the input function is called.
///
/// # Parameters
///
/// - `counters` - Shared counters for tracking message flow across threads.
///
fn on_input_function_called(counters: &MessageCounters) {
    counters.increment_vmm_thread_input_calls();

    // Sanity check that no messages are lost.
    #[cfg(debug_assertions)]
    {
        // The following check is not atomic, but since the two counters are monotonically
        // increasing AND they are strictly updated one after another, it should be sufficient to
        // detect message losses.

        let cached_mem_thread_num_messages_received: usize =
            counters.get_mem_thread_messages_received();
        let cached_vmm_thread_num_input_calls: usize = counters.get_vmm_thread_input_calls();

        debug_assert!(
            cached_vmm_thread_num_input_calls <= cached_mem_thread_num_messages_received,
            "vmm thread has called the input function more times than the memory thread has \
             received messages ({} > {})",
            cached_vmm_thread_num_input_calls,
            cached_mem_thread_num_messages_received
        );
    }
}

///
/// # Description
///
/// Handler to be called whenever a message is received from the memory thread.
///
/// # Parameters
///
/// - `counters` - Shared counters for tracking message flow across threads.
///
fn on_message_received_from_memory_thread(counters: &MessageCounters) {
    counters.increment_vmm_thread_messages_received();

    // Sanity check that no messages are lost.
    #[cfg(debug_assertions)]
    {
        // The following check is not atomic, but since the two counters are monotonically
        // increasing AND they are strictly updated one after another, it should be sufficient to
        // detect message losses.

        let cached_vmm_thread_num_input_calls: usize = counters.get_vmm_thread_input_calls();
        let cached_vmm_thread_num_messages_received: usize =
            counters.get_vmm_thread_messages_received();

        debug_assert!(
            cached_vmm_thread_num_messages_received <= cached_vmm_thread_num_input_calls,
            "vmm thread has received more messages than it has called the input function ({} > {})",
            cached_vmm_thread_num_messages_received,
            cached_vmm_thread_num_input_calls
        );
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::get_stderr_writer;
    use ::anyhow::{
        Result as AnyResult,
        anyhow,
    };
    use ::std::{
        env,
        fs,
        fs::Metadata,
        io::Write,
        path::PathBuf,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    fn unique_log_path(suffix: &str) -> AnyResult<(String, PathBuf)> {
        let mut path: PathBuf = env::temp_dir();
        let nanos: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| anyhow!("failed to compute timestamp (error={:?})", error))?
            .as_nanos();
        let file_name: String = format!("nanvix-uservm-{suffix}-{nanos}.log");
        path.push(&file_name);
        Ok((path.to_string_lossy().into_owned(), path))
    }

    #[test]
    fn get_stderr_writer_creates_target_file() -> AnyResult<()> {
        let (path_str, path_buf): (String, PathBuf) = unique_log_path("stderr")?;

        {
            let mut writer: Box<dyn Write + Send> = get_stderr_writer(Some(path_str.clone()))?;
            writer.write_all(b"hello stderr")?;
            writer.flush()?;
        }

        let metadata: Metadata = fs::metadata(&path_buf)?;
        assert!(metadata.is_file(), "expected log path to be a regular file");
        assert!(metadata.len() >= b"hello stderr".len() as u64, "log file is unexpectedly empty");

        fs::remove_file(path_buf).ok();
        Ok(())
    }

    #[test]
    fn get_stderr_writer_errors_when_directory_missing() {
        let nanos: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("timestamp")
            .as_nanos();
        let mut file_path: PathBuf = env::temp_dir();
        file_path.push(format!("nanvix-uservm-missing-{nanos}"));
        file_path.push("stderr.log");

        let result: AnyResult<Box<dyn Write + Send>> =
            get_stderr_writer(Some(file_path.to_string_lossy().into_owned()));
        assert!(result.is_err(), "expected failure when parent directory does not exist");
    }
}
