// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! High-level orchestration layer for user virtual machines.
//!
//! This crate wires together the asynchronous runtime, virtual machine monitor (VMM), and
//! supporting worker threads that collectively emulate a user virtual machine. It exposes a public
//! API used by standalone host components to spawn, supervise, and interact with UserVM instances.

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
pub mod guest_profiler;
pub mod memory_thread;
pub mod orchestrator;
pub mod pal;
#[cfg(feature = "profile-time")]
pub mod perf;
pub mod standalone;
pub mod vmm;

//==================================================================================================
// Imports
//==================================================================================================

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
    warn,
};
#[cfg(feature = "profile-time")]
use ::std::time::Instant;
use ::std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    fs::File,
    io::Write,
    sync::Arc,
    time::Duration,
};
use ::sys::{
    ipc::{
        DataChunk,
        DataChunkHeader,
        GuestSgBulkHeader,
        GuestSgBulkKind,
        GuestSgSegment,
        HostBulkTransferHeader,
        IkcFrame,
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
        SG_BULK_MAX_BYTES,
        SG_BULK_MAX_SEGMENTS,
        VmBusMessage,
        VmBusMessageKind,
    },
    mm::Address,
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
    time::timeout,
};

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Maximum number messages that can be queued in a channel.
///
pub const CHANNEL_CAPACITY: usize = 1024;

///
/// # Description
///
/// Grace period for a post-vCPU teardown worker (the orchestrator task or the memory thread) to
/// finish once the guest has exited. These workers only need to observe the shutdown signal and
/// unwind, so they normally terminate near-instantly; bounding the join guarantees that a worker
/// which fails to terminate cannot keep the VMM task — and therefore the uservm/`nanvixd` process —
/// alive forever. The in-process shutdown watchdog in [`standalone::StandaloneVmHandle`] only fires
/// after this VMM task completes, so without this bound a wedged teardown worker would leave
/// `nanvixd` hung with no stdout and no exit (the rare standalone interactive boot/teardown wedge).
///
const TEARDOWN_JOIN_GRACE: Duration = Duration::from_secs(5);

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone)]
struct SgSegmentRange {
    data_gpa: u32,
    data_len: u32,
}

struct PendingSgPull {
    total_len: usize,
    segments: Vec<SgSegmentRange>,
}

#[derive(Default)]
struct SgBulkRegistry {
    next_transfer_id: u32,
    pending: BTreeMap<u32, PendingSgPull>,
}

impl SgBulkRegistry {
    fn insert(&mut self, pending: PendingSgPull, first_valid_transfer_id: u32) -> Result<u32> {
        let first_valid_transfer_id: u32 = first_valid_transfer_id.max(1);
        let mut transfer_id: u32 = if self.next_transfer_id < first_valid_transfer_id
            || self.next_transfer_id == u32::MAX
        {
            first_valid_transfer_id
        } else {
            self.next_transfer_id + 1
        };

        for _ in first_valid_transfer_id..=u32::MAX {
            if !self.pending.contains_key(&transfer_id) {
                self.next_transfer_id = transfer_id;
                self.pending.insert(transfer_id, pending);
                return Ok(transfer_id);
            }
            transfer_id = if transfer_id == u32::MAX {
                first_valid_transfer_id
            } else {
                transfer_id + 1
            };
        }

        anyhow::bail!("scatter/gather transfer id space exhausted")
    }

    fn remove(&mut self, transfer_id: u32) -> Option<PendingSgPull> {
        self.pending.remove(&transfer_id)
    }
}

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
    /// Optional string of kernel arguments written to guest control registers.
    pub kernel_args: Option<String>,
    /// Optional path to a RAM filesystem image exposed to the guest.
    pub ramfs_filename: Option<String>,
    /// Optional path to a file used to capture the guest's stderr stream.
    pub stderr: Option<String>,
    /// Channel used to forward port-I/O writes (messages and data chunk transfers) from the guest.
    pub vcpu_thread_stdout_tx: Sender<IkcFrame>,
    /// Channel providing transfers emitted by the host I/O handler destined for the guest.
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

        let sg_bulk_registry: Arc<Mutex<SgBulkRegistry>> =
            Arc::new(Mutex::new(SgBulkRegistry::default()));

        // Move the stdout sender out of args so no extra clone keeps the
        // data_rx channel alive after the VMM thread finishes.
        // Output function used for emulating I/O port writes.
        let vmm_stdout_fn: Box<StdoutFn> =
            output_fn(args.vcpu_thread_stdout_tx, sg_bulk_registry.clone());

        // Input function used for emulating I/O port reads.
        let ikc_pending: std::sync::Arc<std::sync::atomic::AtomicBool> =
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let vmm_stdin_fn: Box<StdinFn> = build_input_fn(
            vcpu_thread_stdin_rx,
            args.counters.clone(),
            ikc_pending.clone(),
            sg_bulk_registry,
        );

        #[cfg(feature = "profile-time")]
        #[allow(clippy::cast_possible_truncation)]
        perf_timings.set_channel_setup(channel_setup_start.elapsed().as_micros() as u64);

        let mut microvm: Vmm = Vmm::new(MicroVmArgs {
            input: vmm_stdin_fn,
            output: vmm_stdout_fn,
            stderr: vmm_stderr_fn,
            control_rx: vcpu_thread_control_rx,
            control_tx: vcpu_thread_control_tx,
            kernel_filename: args.kernel_filename,
            initrd_filename: args.initrd_filename.clone(),
            initrd_args: args.initrd_args.clone(),
            kernel_args: args.kernel_args.clone(),
            ramfs_filename: args.ramfs_filename.clone(),
            restoring_from_snapshot: args.snapshot_path.is_some(),
            ikc_pending: ikc_pending.clone(),
            #[cfg(feature = "gdb")]
            gdb_port: args.gdb_port,
            #[cfg(feature = "profile-time")]
            perf_timings: perf_timings.clone(),
        })?;

        // If a snapshot path is provided, restore VM state from the snapshot.
        if let Some(snapshot_path) = args.snapshot_path {
            #[cfg(feature = "profile-time")]
            let snapshot_restore_start: Instant = Instant::now();

            microvm.load_snapshot(snapshot_path).await?;

            #[cfg(feature = "profile-time")]
            #[allow(clippy::cast_possible_truncation)]
            perf_timings.set_snapshot_restore(snapshot_restore_start.elapsed().as_micros() as u64);
        }

        // Enable guest profiler if requested.
        let guest_profiler = if args_guest_profile_path.is_some() {
            Some(microvm.enable_guest_profiler())
        } else {
            None
        };

        // Start host kernel tracing session if profiling is enabled.
        // If an error causes an early return below, the session's Drop impl
        // cancels the trace (via `wpr -cancel`) which is the desired cleanup
        // behavior — we don't want a stale WPR session left running.
        let mut kernel_session = if args_guest_profile_path.is_some() {
            let trace_path = format!(
                "{}.{}",
                args_guest_profile_path
                    .as_deref()
                    .unwrap_or("guest-profile"),
                if cfg!(target_os = "windows") {
                    "etl"
                } else {
                    "perf.data"
                }
            );
            let mut session = crate::guest_profiler::HostKernelSession::new(&trace_path);
            match session.start() {
                Ok(()) => Some(session),
                Err(e) => {
                    warn!("Host kernel tracing failed to start: {e}");
                    None
                },
            }
        } else {
            None
        };

        let vmem: Arc<Mutex<VirtualMemory>> = microvm.vmem();
        let guest: Arc<Mutex<Guest>> = microvm.guest();

        // Phase: Thread spawning.
        #[cfg(feature = "profile-time")]
        let thread_spawn_start: Instant = Instant::now();

        // Create a thread that reads from vm_rx and writes to vm_rx2.
        let memory_thread: MemoryThread = MemoryThread::new(
            args.memory_thread_data_rx,
            memory_thread_data_tx,
            memory_thread_control_rx,
            memory_thread_control_tx,
            add_credit_fn(guest.clone(), vmem.clone(), microvm.ikc_notifier()),
            args.counters.clone(),
        );
        let mut memory_thread: JoinHandle<()> = memory_thread.spawn();

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

        let mut orchestrator_thread_handle: JoinHandle<Result<()>> = orchestrator_thread.spawn();

        #[cfg(feature = "profile-time")]
        #[allow(clippy::cast_possible_truncation)]
        perf_timings.set_thread_spawn(thread_spawn_start.elapsed().as_micros() as u64);

        let exit_code: Result<u16> = match vmm_thread.await {
            Ok(exit_code) => exit_code,
            Err(error) => {
                let reason: String = format!("failed to join vmm thread (error={error:?})");
                error!("spawn(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // The guest/vCPU has exited (`vmm_thread` resolved above). The orchestrator and memory
        // worker tasks should now observe the shutdown signal and unwind promptly, so bound each
        // join with a grace period: a worker that fails to terminate must not keep this VMM task —
        // and therefore the uservm/`nanvixd` process — alive forever (the shutdown watchdog in
        // `standalone::StandaloneVmHandle` only fires once this task completes). If a join exceeds
        // the grace period, abort the task; the guest is already gone, so its only remaining work is
        // cleanup that is safe to drop.
        match timeout(TEARDOWN_JOIN_GRACE, &mut orchestrator_thread_handle).await {
            // Joined; the orchestrator's run result is intentionally not propagated, matching the
            // prior behavior of this teardown path.
            Ok(Ok(_)) => {},
            Ok(Err(error)) => {
                error!("spawn(): failed to join orchestrator thread (error={error:?})");
            },
            Err(_elapsed) => {
                warn!(
                    "spawn(): orchestrator thread did not terminate within {:?} after guest exit; \
                     aborting to avoid wedging VM teardown",
                    TEARDOWN_JOIN_GRACE
                );
                orchestrator_thread_handle.abort();
            },
        }

        match timeout(TEARDOWN_JOIN_GRACE, &mut memory_thread).await {
            Ok(Ok(())) => {},
            Ok(Err(error)) => {
                error!("spawn(): error joining memory thread (error={error:?})");
            },
            Err(_elapsed) => {
                warn!(
                    "spawn(): memory thread did not terminate within {:?} after guest exit; \
                     aborting to avoid wedging VM teardown",
                    TEARDOWN_JOIN_GRACE
                );
                memory_thread.abort();
            },
        }

        #[cfg(feature = "profile-time")]
        #[allow(clippy::cast_possible_truncation)]
        perf_timings.set_total(run_start.elapsed().as_micros() as u64);

        // Stop host kernel trace session before post-processing so the
        // trace does not include symbol resolution / file I/O work.
        if let Some(ref mut session) = kernel_session {
            match session.stop() {
                Ok(trace_path) => {
                    eprintln!("PROFILER: host trace saved to {}", trace_path);
                },
                Err(e) => warn!("Host kernel session stop failed: {e}"),
            }
        }

        // Write guest profiler folded stacks if profiling was enabled.
        if let (Some(profiler), Some(path)) = (guest_profiler, &args_guest_profile_path) {
            let mut sym_paths: Vec<std::path::PathBuf> = Vec::new();
            if let Ok(p) = std::env::var("NANVIX_KERNEL_SYMBOLS") {
                sym_paths.push(std::path::PathBuf::from(p));
            }
            if let Ok(p) = std::env::var("NANVIX_USER_SYMBOLS") {
                sym_paths.push(std::path::PathBuf::from(p));
            }
            let sym_refs: Vec<&std::path::Path> = sym_paths.iter().map(|p| p.as_path()).collect();
            let resolver = crate::guest_profiler::SymbolResolver::from_elf_files(&sym_refs);

            // Drain samples once and use for both folded output and timestamp log.
            let guest_sample_vec = profiler.drain_samples();

            // Write folded stacks from the drained samples.
            if let Err(e) = profiler
                .write_folded_from_samples(path, &guest_sample_vec, |addr| resolver.resolve(addr))
            {
                error!("Failed to write guest profile: {e:?}");
            } else {
                eprintln!("GUEST_PROFILE: wrote {} samples to {}", guest_sample_vec.len(), path);
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
    Box::new(move |vcpu_tid| {
        vmm.request_shutdown(vcpu_tid);
    })
}

fn add_credit_fn(
    guest: Arc<Mutex<Guest>>,
    vmem: Arc<Mutex<VirtualMemory>>,
    notifier: crate::vmm::IkcNotifier,
) -> Box<AddCreditFn> {
    Box::new(move || {
        let guest: Arc<Mutex<Guest>> = guest.clone();
        let vmem: Arc<Mutex<VirtualMemory>> = vmem.clone();
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
            notifier.notify()?;
            Ok(())
        })
    })
}

struct ValidatedSgBulk {
    source_pid: ::sys::pm::ProcessIdentifier,
    source_tid: ::sys::pm::ThreadIdentifier,
    destination_pid: ::sys::pm::ProcessIdentifier,
    destination_tid: ::sys::pm::ThreadIdentifier,
    kind: GuestSgBulkKind,
    total_len: usize,
    tag: u32,
    segments: Vec<SgSegmentRange>,
}

fn validate_guest_range(vmem: &VirtualMemory, addr: u64, len: usize) -> Result<()> {
    let addr: usize = usize::try_from(addr).map_err(|e| {
        let reason: String = format!("guest address does not fit usize: {e}");
        error!("{reason}");
        anyhow::Error::msg(reason)
    })?;
    match addr.checked_add(len) {
        Some(end) if end <= vmem.get_size() => Ok(()),
        _ => {
            let reason: String =
                format!("invalid guest physical range (addr={addr:#x}, len={len:#x})");
            error!("{reason}");
            anyhow::bail!(reason)
        },
    }
}

fn read_guest_sg_segment(vmem: &VirtualMemory, addr: u64) -> Result<GuestSgSegment> {
    validate_guest_range(vmem, addr, GuestSgSegment::SIZE)?;
    let mut bytes: [u8; GuestSgSegment::SIZE] = [0u8; GuestSgSegment::SIZE];
    vmem.read_bytes(addr, &mut bytes)?;
    GuestSgSegment::try_from_bytes(bytes).map_err(|e| {
        let reason: String = format!("failed to parse scatter/gather segment: {e:?}");
        error!("{reason}");
        anyhow::Error::msg(reason)
    })
}

fn read_guest_sg_header(vmem: &VirtualMemory, addr: u64) -> Result<GuestSgBulkHeader> {
    validate_guest_range(vmem, addr, GuestSgBulkHeader::SIZE)?;
    let mut bytes: [u8; GuestSgBulkHeader::SIZE] = [0u8; GuestSgBulkHeader::SIZE];
    vmem.read_bytes(addr, &mut bytes)?;
    GuestSgBulkHeader::try_from_bytes(bytes).map_err(|e| {
        let reason: String = format!("failed to parse scatter/gather header: {e:?}");
        error!("{reason}");
        anyhow::Error::msg(reason)
    })
}

fn parse_guest_sg_bulk(vmem: &VirtualMemory, header_addr: u64) -> Result<ValidatedSgBulk> {
    let header: GuestSgBulkHeader = read_guest_sg_header(vmem, header_addr)?;
    let kind: GuestSgBulkKind = header.kind().map_err(|e| {
        let reason: String = format!("invalid scatter/gather transfer kind: {e:?}");
        error!("{reason}");
        anyhow::Error::msg(reason)
    })?;
    let segment_count: usize = header.segment_count().get() as usize;
    let total_len: usize = header.total_len() as usize;

    if total_len == 0 || total_len > SG_BULK_MAX_BYTES {
        let reason: String = format!("invalid scatter/gather length (len={total_len})");
        error!("{reason}");
        anyhow::bail!(reason)
    }
    if segment_count == 0 || segment_count > SG_BULK_MAX_SEGMENTS as usize {
        let reason: String =
            format!("invalid scatter/gather segment count (count={segment_count})");
        error!("{reason}");
        anyhow::bail!(reason)
    }

    let mut segments: Vec<SgSegmentRange> = Vec::with_capacity(segment_count);
    let mut seen_descriptors: BTreeSet<u32> = BTreeSet::new();
    let mut cumulative_len: usize = 0;
    let mut segment: GuestSgSegment = header.first();

    for index in 0..segment_count {
        let data_len: usize = segment.data_len() as usize;
        if data_len == 0 {
            let reason: String = format!("zero-length scatter/gather segment (index={index})");
            error!("{reason}");
            anyhow::bail!(reason)
        }
        cumulative_len = cumulative_len.checked_add(data_len).ok_or_else(|| {
            let reason: String = "scatter/gather cumulative length overflow".to_string();
            error!("{reason}");
            anyhow::Error::msg(reason)
        })?;
        if cumulative_len > total_len || cumulative_len > SG_BULK_MAX_BYTES {
            let reason: String =
                format!("scatter/gather segment lengths exceed declared length (index={index})");
            error!("{reason}");
            anyhow::bail!(reason)
        }
        validate_guest_range(vmem, segment.data_gpa() as u64, data_len)?;
        segments.push(SgSegmentRange {
            data_gpa: segment.data_gpa(),
            data_len: segment.data_len(),
        });

        let next: u32 = u32::try_from(segment.next().into_raw_value()).map_err(|e| {
            let reason: String =
                format!("scatter/gather next descriptor address does not fit u32: {e}");
            error!("{reason}");
            anyhow::Error::msg(reason)
        })?;
        if index + 1 == segment_count {
            if next != 0 {
                let reason: String = "scatter/gather chain has extra descriptors".to_string();
                error!("{reason}");
                anyhow::bail!(reason)
            }
        } else {
            if next == 0 {
                let reason: String = "scatter/gather chain ended early".to_string();
                error!("{reason}");
                anyhow::bail!(reason)
            }
            if !seen_descriptors.insert(next) {
                let reason: String =
                    format!("scatter/gather descriptor cycle detected (addr={next:#x})");
                error!("{reason}");
                anyhow::bail!(reason)
            }
            segment = read_guest_sg_segment(vmem, next as u64)?;
        }
    }

    if cumulative_len != total_len {
        let reason: String = format!(
            "scatter/gather length mismatch (declared={total_len}, actual={cumulative_len})"
        );
        error!("{reason}");
        anyhow::bail!(reason)
    }

    Ok(ValidatedSgBulk {
        source_pid: header.source_pid(),
        source_tid: header.source_tid(),
        destination_pid: header.destination_pid(),
        destination_tid: header.destination_tid(),
        kind,
        total_len,
        tag: header.tag(),
        segments,
    })
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Obtains a writer for the virtual machine's standard error device.
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
/// On success, the function returns a writer for the virtual machine's standard error
/// stream. An error is returned when the target file cannot be created.
///
pub fn get_stderr_writer(vm_stderr: Option<String>) -> Result<Box<dyn Write + Send>> {
    // Obtain a writer for the virtual machine's standard error device.
    let file_writer: Box<dyn Write + Send> = if let Some(vm_stderr) = vm_stderr {
        // On Windows the path is first inspected via `is_windows_named_pipe`: if it points at a
        // named pipe (`\\.\pipe\…` or `\\?\pipe\…`) the file is opened in write-only mode
        // without `create`/`truncate`, because Windows' `CreateFile` rejects those flags on a
        // named pipe with `ERROR_INVALID_PARAMETER` (os error 87).
        #[cfg(windows)]
        if is_windows_named_pipe(&vm_stderr) {
            let file: File = File::options().read(false).write(true).open(&vm_stderr)?;
            return Ok(Box::new(file));
        }
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
/// Returns whether the given path points at a Windows named pipe.
///
/// Recognises both the canonical `\\.\pipe\` prefix and the rarer long-path `\\?\pipe\` form.
/// Matching is case-insensitive on the prefix only -- Windows accepts pipe paths like
/// `\\.\PIPE\foo`, `\\.\Pipe\foo`, etc. as equivalent to the lower-case form -- but the pipe name
/// itself is forwarded verbatim to `CreateFile`.
///
/// # Out of scope
///
/// Remote UNC pipe paths (`\\<server>\pipe\<name>`) and forward-slash variants (`//./pipe/<name>`)
/// are intentionally not recognised: the in-tree callers (the containerd shim and friends) always
/// supply local-machine, backslash- canonical paths. If a future caller needs remote pipes, the
/// helper should be extended together with the test suite below.
///
/// # Parameters
///
/// - `path`: Path to inspect.
///
/// # Returns
///
/// `true` if the path begins with a Windows named-pipe prefix, `false` otherwise.
///
#[cfg(windows)]
fn is_windows_named_pipe(path: &str) -> bool {
    // Only inspect the prefix bytes: pipe names themselves should be passed verbatim to
    // `CreateFile`, and large paths should not allocate just to be classified.
    const PREFIX: &[u8] = br"\\.\pipe\";
    const LONG_PREFIX: &[u8] = br"\\?\pipe\";
    // Both recognised prefixes must have the same length so the single `head` slice below can be
    // compared against either of them.
    ::static_assert::assert_eq!(PREFIX.len() == LONG_PREFIX.len());
    const PREFIX_LEN: usize = PREFIX.len();
    let bytes: &[u8] = path.as_bytes();
    if bytes.len() < PREFIX_LEN {
        return false;
    }
    let head: &[u8] = &bytes[..PREFIX_LEN];
    head.eq_ignore_ascii_case(PREFIX) || head.eq_ignore_ascii_case(LONG_PREFIX)
}

///
/// # Description
///
/// Builds an input callback that delivers messages from the host I/O handler to the virtual
/// machine.
///
/// # Parameters
///
/// - `input_queue` - Channel used to receive emulated port-I/O requests originating from the host
///   I/O handler.
/// - `counters` - Shared counters for tracking message flow across threads.
///
/// # Returns
///
/// A boxed closure compatible with the VMM's stdin handler implementation.
///
fn build_input_fn(
    mut input_queue: Receiver<IkcFrame>,
    counters: MessageCounters,
    ikc_pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
    sg_bulk_registry: Arc<Mutex<SgBulkRegistry>>,
) -> Box<StdinFn> {
    // Input function used for emulating I/O port reads.
    let input = move |guest: &Arc<Mutex<Guest>>,
                      vmem: &Arc<Mutex<VirtualMemory>>,
                      data,
                      size|
          -> Result<()> {
        use ::std::mem;
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
                        on_message_received_from_memory_thread(&counters);
                        msg.message_type = MessageType::Ikc;
                        let mut locked_guest: MutexGuard<'_, Guest> = guest.blocking_lock();
                        let mut locked_vm: MutexGuard<'_, VirtualMemory> = vmem.blocking_lock();

                        locked_vm.write_bytes(data as u64, &msg.to_bytes())?;
                        locked_guest.consume_credit(&mut locked_vm)?;
                        ikc_pending.store(false, std::sync::atomic::Ordering::Release);
                    },
                    IkcFrame::Bulk(bulk) => {
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
                        let mut locked_guest: MutexGuard<'_, Guest> = guest.blocking_lock();
                        let mut locked_vm: MutexGuard<'_, VirtualMemory> = vmem.blocking_lock();

                        let actual_len: usize = bulk.data().len();
                        let transfer_id: u32 = bulk.header().data_addr();
                        let pending_pull: Option<PendingSgPull> =
                            sg_bulk_registry.blocking_lock().remove(transfer_id);
                        if let Some(pending_pull) = pending_pull {
                            if actual_len > pending_pull.total_len {
                                let reason: String = format!(
                                    "bulk response exceeds pending scatter/gather pull length \
                                     (actual={actual_len}, max={})",
                                    pending_pull.total_len
                                );
                                error!("{reason}");
                                anyhow::bail!(reason);
                            }
                            trace!(
                                "input(): scattering {actual_len} bulk bytes to guest \
                                 (transfer_id={transfer_id})"
                            );
                            let mut copied: usize = 0;
                            for segment in &pending_pull.segments {
                                if copied == actual_len {
                                    break;
                                }
                                let segment_len: usize = segment.data_len as usize;
                                let bytes_to_copy: usize =
                                    core::cmp::min(segment_len, actual_len - copied);
                                locked_vm.write_bytes(
                                    segment.data_gpa as u64,
                                    &bulk.data()[copied..copied + bytes_to_copy],
                                )?;
                                copied += bytes_to_copy;
                            }
                        } else {
                            let dest_addr: u64 = transfer_id as u64;
                            let guest_size: usize = locked_vm.get_size();
                            let dest_addr_usize: usize =
                                usize::try_from(dest_addr).map_err(|e| {
                                    let reason: String =
                                        format!("bulk response address does not fit usize: {e}");
                                    error!("input(): {reason}");
                                    anyhow::Error::msg(reason)
                                })?;
                            if dest_addr_usize >= guest_size {
                                let reason: String = format!(
                                    "bulk response references unknown scatter/gather transfer id \
                                     or invalid legacy address (data_addr={dest_addr:#x}, \
                                     guest_size={guest_size:#x})"
                                );
                                error!("input(): {reason}");
                                anyhow::bail!(reason);
                            }
                            validate_guest_range(&locked_vm, dest_addr, actual_len)?;
                            trace!(
                                "input(): writing {actual_len} legacy bulk bytes to guest at \
                                 {dest_addr:#x}"
                            );
                            locked_vm.write_bytes(dest_addr, bulk.data())?;
                        }

                        // Construct a PullResponse notification message. The kernel's
                        // main loop reads this from the regular message buffer, detects the
                        // message type, and wakes the sleeping pull thread.
                        let completion_header: DataChunkHeader = (*bulk.header()).with_data_len(
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

///
/// # Description
///
/// Builds an output callback that forwards messages and data chunk transfers emitted by the virtual
/// machine. The callback inspects the [`VmBusMessage`] to determine whether the guest is
/// sending a standard IKC message or a bulk transfer and constructs the appropriate [`IkcFrame`].
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
fn output_fn(
    queue: Sender<IkcFrame>,
    sg_bulk_registry: Arc<Mutex<SgBulkRegistry>>,
) -> Box<StdoutFn> {
    // On Windows/WHP the closure below runs on the vCPU thread while it holds the WHP partition
    // lock (held across `Emulator::handle_pmio_access()`). Sending directly on the bounded channel
    // there would block the vCPU thread under the lock whenever the asynchronous consumer is
    // momentarily starved, freezing the guest (issue #2603). `IkcStdoutSink` interposes a dedicated
    // forwarding thread on Windows so the under-lock enqueue never blocks (lossless hand-off;
    // back-pressure applied off the partition lock). On Linux the frames are sent directly.
    let sink: crate::pal::IkcStdoutSink = crate::pal::IkcStdoutSink::new(queue);

    // Output function used for emulating I/O port writes.
    let output = move |vm: &Arc<Mutex<VirtualMemory>>, envelope: &VmBusMessage| -> Result<()> {
        use ::std::mem;

        let envelope_kind: VmBusMessageKind = envelope.kind().map_err(|e| {
            let reason: String = format!("invalid vmbus message kind: {e:?}");
            error!("output(): {reason}");
            anyhow::Error::msg(reason)
        })?;
        match envelope_kind {
            VmBusMessageKind::Ikc => {
                // Standard IKC message: read the message from guest memory.
                let mut bytes: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
                trace!("output(): reading message from user VM");
                vm.blocking_lock()
                    .read_bytes(envelope.message_addr() as u64, &mut bytes)?;

                let message: Message = match Message::try_from_bytes(bytes) {
                    Ok(message) => message,
                    Err(err) => {
                        let reason: String = format!("failed to parse message: {err:?}");
                        error!("output(): {reason}");
                        anyhow::bail!(reason);
                    },
                };

                trace!("output(): forwarding message to system VM");
                if let Err(e) = sink.send(IkcFrame::Message(message)) {
                    let reason: String = format!("failed to send message: {e:?}");
                    error!("output(): {reason}");
                    anyhow::bail!(reason);
                }

                trace!("output(): message forwarded to system VM");
            },
            VmBusMessageKind::LegacyBulk => {
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

                let bulk: DataChunk = DataChunk::new(header, data);

                trace!("output(): forwarding data chunk transfer ({data_len} bytes)");
                if let Err(e) = sink.send(IkcFrame::Bulk(bulk)) {
                    let reason: String = format!("failed to send data chunk transfer: {e:?}");
                    error!("output(): {reason}");
                    anyhow::bail!(reason);
                }
            },
            VmBusMessageKind::GuestSgBulk => {
                let (sg_bulk, first_valid_transfer_id): (ValidatedSgBulk, u32) = {
                    let locked_vm: MutexGuard<'_, VirtualMemory> = vm.blocking_lock();
                    let sg_bulk: ValidatedSgBulk =
                        parse_guest_sg_bulk(&locked_vm, envelope.message_addr() as u64)?;
                    let first_valid_transfer_id: u32 = u32::try_from(locked_vm.get_size())
                        .map_err(|e| {
                            let reason: String =
                                format!("guest memory size exceeds transfer id space: {e}");
                            error!("{reason}");
                            anyhow::Error::msg(reason)
                        })?;
                    (sg_bulk, first_valid_transfer_id)
                };

                match sg_bulk.kind {
                    GuestSgBulkKind::Push => {
                        let mut data: Vec<u8> = vec![0u8; sg_bulk.total_len];
                        {
                            let locked_vm: MutexGuard<'_, VirtualMemory> = vm.blocking_lock();
                            let mut offset: usize = 0;
                            for segment in &sg_bulk.segments {
                                let segment_len: usize = segment.data_len as usize;
                                locked_vm.read_bytes(
                                    segment.data_gpa as u64,
                                    &mut data[offset..offset + segment_len],
                                )?;
                                offset += segment_len;
                            }
                        }

                        let header: HostBulkTransferHeader = HostBulkTransferHeader::new_tagged(
                            sg_bulk.source_pid,
                            sg_bulk.source_tid,
                            sg_bulk.destination_pid,
                            sg_bulk.destination_tid,
                            0,
                            u32::try_from(sg_bulk.total_len).map_err(|e| {
                                let reason: String =
                                    format!("scatter/gather length exceeds u32: {e}");
                                error!("{reason}");
                                anyhow::Error::msg(reason)
                            })?,
                            sg_bulk.tag,
                        );
                        let bulk: DataChunk = DataChunk::new(header, data);

                        trace!(
                            "output(): forwarding scatter/gather push ({} bytes, {} segments)",
                            sg_bulk.total_len,
                            sg_bulk.segments.len()
                        );
                        if let Err(e) = sink.send(IkcFrame::Bulk(bulk)) {
                            let reason: String =
                                format!("failed to send scatter/gather transfer: {e:?}");
                            error!("output(): {reason}");
                            anyhow::bail!(reason);
                        }
                    },
                    GuestSgBulkKind::Pull => {
                        let pending: PendingSgPull = PendingSgPull {
                            total_len: sg_bulk.total_len,
                            segments: sg_bulk.segments,
                        };
                        let transfer_id: u32 = sg_bulk_registry
                            .blocking_lock()
                            .insert(pending, first_valid_transfer_id)?;
                        let header: HostBulkTransferHeader = HostBulkTransferHeader::new_tagged(
                            sg_bulk.source_pid,
                            sg_bulk.source_tid,
                            sg_bulk.destination_pid,
                            sg_bulk.destination_tid,
                            transfer_id,
                            u32::try_from(sg_bulk.total_len).map_err(|e| {
                                let reason: String =
                                    format!("scatter/gather length exceeds u32: {e}");
                                error!("{reason}");
                                anyhow::Error::msg(reason)
                            })?,
                            sg_bulk.tag,
                        );
                        let bulk: DataChunk = DataChunk::new(header, Vec::new());

                        trace!(
                            "output(): forwarding scatter/gather pull request \
                             (transfer_id={transfer_id}, len={})",
                            sg_bulk.total_len
                        );
                        if let Err(e) = sink.send(IkcFrame::Bulk(bulk)) {
                            sg_bulk_registry.blocking_lock().remove(transfer_id);
                            let reason: String =
                                format!("failed to send scatter/gather pull request: {e:?}");
                            error!("output(): {reason}");
                            anyhow::bail!(reason);
                        }
                    },
                }
            },
            VmBusMessageKind::KlogBlock => {
                // Kernel-log blocks are delivered on `DEFAULT_KLOG_PORT` and handled by the console
                // path, never the application stdout path. Receiving one here means a guest or port
                // mix-up, so fail loudly rather than misrouting kernel log bytes into application
                // output.
                let reason: String =
                    "unexpected KlogBlock envelope on the application stdout port".to_string();
                error!("output(): {reason}");
                anyhow::bail!(reason);
            },
        }

        Ok(())
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

    #[cfg(windows)]
    #[test]
    fn is_windows_named_pipe_recognises_canonical_prefix() {
        use super::is_windows_named_pipe;
        assert!(is_windows_named_pipe(r"\\.\pipe\foo"));
        assert!(is_windows_named_pipe(r"\\.\pipe\containerd-shim-abc"));
    }

    #[cfg(windows)]
    #[test]
    fn is_windows_named_pipe_recognises_long_path_prefix() {
        use super::is_windows_named_pipe;
        assert!(is_windows_named_pipe(r"\\?\pipe\foo"));
    }

    #[cfg(windows)]
    #[test]
    fn is_windows_named_pipe_rejects_regular_paths() {
        use super::is_windows_named_pipe;
        assert!(!is_windows_named_pipe(r"C:\temp\foo.log"));
        assert!(!is_windows_named_pipe(r"foo.log"));
        assert!(!is_windows_named_pipe(r"\\server\share\foo"));
        assert!(!is_windows_named_pipe(""));
    }

    #[cfg(windows)]
    #[test]
    fn is_windows_named_pipe_requires_trailing_separator() {
        use super::is_windows_named_pipe;
        // The prefix must include the trailing backslash; "\\.\pipefoo" is not a pipe path.
        assert!(!is_windows_named_pipe(r"\\.\pipefoo"));
        assert!(!is_windows_named_pipe(r"\\?\pipefoo"));
    }

    #[cfg(windows)]
    #[test]
    fn is_windows_named_pipe_recognises_case_variants() {
        use super::is_windows_named_pipe;
        // Windows treats pipe paths as case-insensitive: callers may legitimately
        // pass mixed-case prefixes.
        assert!(is_windows_named_pipe(r"\\.\PIPE\foo"));
        assert!(is_windows_named_pipe(r"\\.\Pipe\containerd-shim"));
        assert!(is_windows_named_pipe(r"\\?\PIPE\foo"));
        assert!(is_windows_named_pipe(r"\\?\PiPe\foo"));
    }

    #[cfg(windows)]
    #[test]
    fn is_windows_named_pipe_handles_short_inputs() {
        use super::is_windows_named_pipe;
        // Inputs shorter than the prefix must not panic.
        assert!(!is_windows_named_pipe(r"\\.\pipe"));
        assert!(!is_windows_named_pipe(r"\\.\"));
        assert!(!is_windows_named_pipe(r"\"));
    }

    #[cfg(windows)]
    #[test]
    fn is_windows_named_pipe_rejects_out_of_scope_variants() {
        use super::is_windows_named_pipe;
        // Remote UNC pipe paths (\\<server>\pipe\<name>) and forward-slash
        // variants are intentionally out of scope: in-tree callers always
        // supply local-machine, backslash-canonical paths. These tests pin
        // the current behavior so a future contributor extending the helper
        // to cover them also updates this test.
        assert!(!is_windows_named_pipe(r"\\myserver\pipe\foo"));
        assert!(!is_windows_named_pipe("//./pipe/foo"));
        assert!(!is_windows_named_pipe("//?/pipe/foo"));
    }
}
