// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Fine-grained performance timing breakdown for UserVM startup phases.
//!
//! This module is only compiled when the `profile-time` feature is enabled. It provides a
//! thread-safe [`PerfTimings`] struct that records microsecond-precision durations for each
//! phase of the VM lifecycle. After the VM exits, the collected timings are serialized as a
//! single JSON line to host stderr with a `PERF_TIMINGS:` prefix so that `nanvix-bench` can
//! parse and aggregate them.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::sync::{
    Arc,
    atomic::{
        AtomicU64,
        Ordering,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Prefix used to identify performance timing lines on stderr.
pub const PERF_TIMINGS_PREFIX: &str = "PERF_TIMINGS:";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Thread-safe collection of per-phase execution time measurements (in microseconds).
///
/// Each phase is stored in an [`AtomicU64`] so that different threads (VMM, I/O handler,
/// orchestrator) can record their timings independently without locking.
///
#[derive(Clone)]
pub struct PerfTimings {
    /// Channel and internal plumbing setup time.
    channel_setup_us: Arc<AtomicU64>,
    /// Hypervisor partition/VM creation time (KVM or WHP setup, excluding vmem and vCPU).
    partition_create_us: Arc<AtomicU64>,
    /// Virtual memory allocation time.
    vmem_create_us: Arc<AtomicU64>,
    /// vCPU allocation time.
    vcpu_create_us: Arc<AtomicU64>,
    /// Kernel ELF loading time.
    kernel_load_us: Arc<AtomicU64>,
    /// Initrd loading time.
    initrd_load_us: Arc<AtomicU64>,
    /// RamFS loading time.
    ramfs_load_us: Arc<AtomicU64>,
    /// vCPU reset and pvclock setup time.
    vcpu_reset_us: Arc<AtomicU64>,
    /// EPT pre-population time.
    ept_populate_us: Arc<AtomicU64>,
    /// Snapshot restore time (memory and CPU state restoration from snapshot).
    snapshot_restore_us: Arc<AtomicU64>,
    /// Snapshot creation time (memory and CPU state serialization to snapshot files).
    snapshot_creation_us: Arc<AtomicU64>,
    /// Memory thread, VMM thread, and orchestrator spawn time.
    thread_spawn_us: Arc<AtomicU64>,
    /// Cumulative time spent executing guest code (inside the VMM run loop).
    guest_exec_us: Arc<AtomicU64>,
    /// Cumulative time spent in the VMM handling VM exits.
    exit_handling_us: Arc<AtomicU64>,
    /// Total time from `UserVm::run()` entry to VM exit.
    total_us: Arc<AtomicU64>,
    /// Total number of VM exits (all reasons).
    exit_count_total: Arc<AtomicU64>,
    /// Number of PMIO-out exits (fast-path legacy ports).
    exit_count_pmio_out_fast: Arc<AtomicU64>,
    /// Number of PMIO-in exits (fast-path legacy ports).
    exit_count_pmio_in_fast: Arc<AtomicU64>,
    /// Number of PMIO exits dispatched to the emulator (slow-path).
    exit_count_pmio_slow: Arc<AtomicU64>,
    /// Number of Interrupted exits (WHvCancelRunVP).
    exit_count_interrupted: Arc<AtomicU64>,
    /// Number of Halt exits.
    exit_count_halt: Arc<AtomicU64>,
    /// Number of MMIO exits.
    exit_count_mmio: Arc<AtomicU64>,
    /// Number of InterruptWindow exits.
    exit_count_intwin: Arc<AtomicU64>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Default for PerfTimings {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfTimings {
    ///
    /// # Description
    ///
    /// Creates a new [`PerfTimings`] instance with all counters initialized to zero.
    ///
    pub fn new() -> Self {
        Self {
            channel_setup_us: Arc::new(AtomicU64::new(0)),
            partition_create_us: Arc::new(AtomicU64::new(0)),
            vmem_create_us: Arc::new(AtomicU64::new(0)),
            vcpu_create_us: Arc::new(AtomicU64::new(0)),
            kernel_load_us: Arc::new(AtomicU64::new(0)),
            initrd_load_us: Arc::new(AtomicU64::new(0)),
            ramfs_load_us: Arc::new(AtomicU64::new(0)),
            vcpu_reset_us: Arc::new(AtomicU64::new(0)),
            ept_populate_us: Arc::new(AtomicU64::new(0)),
            snapshot_restore_us: Arc::new(AtomicU64::new(0)),
            snapshot_creation_us: Arc::new(AtomicU64::new(0)),
            thread_spawn_us: Arc::new(AtomicU64::new(0)),
            guest_exec_us: Arc::new(AtomicU64::new(0)),
            exit_handling_us: Arc::new(AtomicU64::new(0)),
            total_us: Arc::new(AtomicU64::new(0)),
            exit_count_total: Arc::new(AtomicU64::new(0)),
            exit_count_pmio_out_fast: Arc::new(AtomicU64::new(0)),
            exit_count_pmio_in_fast: Arc::new(AtomicU64::new(0)),
            exit_count_pmio_slow: Arc::new(AtomicU64::new(0)),
            exit_count_interrupted: Arc::new(AtomicU64::new(0)),
            exit_count_halt: Arc::new(AtomicU64::new(0)),
            exit_count_mmio: Arc::new(AtomicU64::new(0)),
            exit_count_intwin: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Records channel and internal plumbing setup time.
    pub fn set_channel_setup(&self, us: u64) {
        self.channel_setup_us.store(us, Ordering::Release);
    }

    /// Records hypervisor partition/VM creation time.
    pub fn set_partition_create(&self, us: u64) {
        self.partition_create_us.store(us, Ordering::Release);
    }

    /// Records virtual memory allocation time.
    pub fn set_vmem_create(&self, us: u64) {
        self.vmem_create_us.store(us, Ordering::Release);
    }

    /// Records vCPU allocation time.
    pub fn set_vcpu_create(&self, us: u64) {
        self.vcpu_create_us.store(us, Ordering::Release);
    }

    /// Records kernel ELF loading time.
    pub fn set_kernel_load(&self, us: u64) {
        self.kernel_load_us.store(us, Ordering::Release);
    }

    /// Records initrd loading time.
    pub fn set_initrd_load(&self, us: u64) {
        self.initrd_load_us.store(us, Ordering::Release);
    }

    /// Records RamFS loading time.
    pub fn set_ramfs_load(&self, us: u64) {
        self.ramfs_load_us.store(us, Ordering::Release);
    }

    /// Records vCPU reset and pvclock setup time.
    pub fn set_vcpu_reset(&self, us: u64) {
        self.vcpu_reset_us.store(us, Ordering::Release);
    }

    /// Records EPT pre-population time.
    pub fn set_ept_populate(&self, us: u64) {
        self.ept_populate_us.store(us, Ordering::Release);
    }

    /// Records snapshot restore time (memory and CPU state restoration from snapshot).
    pub fn set_snapshot_restore(&self, us: u64) {
        self.snapshot_restore_us.store(us, Ordering::Release);
    }

    /// Records snapshot creation time (memory and CPU state serialization to snapshot files).
    pub fn set_snapshot_creation(&self, us: u64) {
        self.snapshot_creation_us.store(us, Ordering::Release);
    }

    /// Records thread spawning time.
    pub fn set_thread_spawn(&self, us: u64) {
        self.thread_spawn_us.store(us, Ordering::Release);
    }

    /// Records cumulative guest execution time.
    pub fn set_guest_exec(&self, us: u64) {
        self.guest_exec_us.store(us, Ordering::Release);
    }

    /// Records cumulative exit handling time.
    pub fn set_exit_handling(&self, us: u64) {
        self.exit_handling_us.store(us, Ordering::Release);
    }

    /// Records total VM execution time.
    pub fn set_total(&self, us: u64) {
        self.total_us.store(us, Ordering::Release);
    }

    /// Records total VM exit count.
    pub fn set_exit_count_total(&self, count: u64) {
        self.exit_count_total.store(count, Ordering::Release);
    }

    /// Records fast-path PMIO-out exit count.
    pub fn set_exit_count_pmio_out_fast(&self, count: u64) {
        self.exit_count_pmio_out_fast
            .store(count, Ordering::Release);
    }

    /// Records fast-path PMIO-in exit count.
    pub fn set_exit_count_pmio_in_fast(&self, count: u64) {
        self.exit_count_pmio_in_fast.store(count, Ordering::Release);
    }

    /// Records slow-path PMIO exit count (emulator dispatch).
    pub fn set_exit_count_pmio_slow(&self, count: u64) {
        self.exit_count_pmio_slow.store(count, Ordering::Release);
    }

    /// Records Interrupted exit count.
    pub fn set_exit_count_interrupted(&self, count: u64) {
        self.exit_count_interrupted.store(count, Ordering::Release);
    }

    /// Records Halt exit count.
    pub fn set_exit_count_halt(&self, count: u64) {
        self.exit_count_halt.store(count, Ordering::Release);
    }

    /// Records MMIO exit count.
    pub fn set_exit_count_mmio(&self, count: u64) {
        self.exit_count_mmio.store(count, Ordering::Release);
    }

    /// Records InterruptWindow exit count.
    pub fn set_exit_count_intwin(&self, count: u64) {
        self.exit_count_intwin.store(count, Ordering::Release);
    }

    ///
    /// # Description
    ///
    /// Serializes the collected timings as a JSON string.
    ///
    pub fn to_json(&self) -> String {
        ::serde_json::json!({
            "channel_setup_us": self.channel_setup_us.load(Ordering::Acquire),
            "partition_create_us": self.partition_create_us.load(Ordering::Acquire),
            "vmem_create_us": self.vmem_create_us.load(Ordering::Acquire),
            "vcpu_create_us": self.vcpu_create_us.load(Ordering::Acquire),
            "kernel_load_us": self.kernel_load_us.load(Ordering::Acquire),
            "initrd_load_us": self.initrd_load_us.load(Ordering::Acquire),
            "ramfs_load_us": self.ramfs_load_us.load(Ordering::Acquire),
            "vcpu_reset_us": self.vcpu_reset_us.load(Ordering::Acquire),
            "ept_populate_us": self.ept_populate_us.load(Ordering::Acquire),
            "snapshot_restore_us": self.snapshot_restore_us.load(Ordering::Acquire),
            "snapshot_creation_us": self.snapshot_creation_us.load(Ordering::Acquire),
            "thread_spawn_us": self.thread_spawn_us.load(Ordering::Acquire),
            "guest_exec_us": self.guest_exec_us.load(Ordering::Acquire),
            "exit_handling_us": self.exit_handling_us.load(Ordering::Acquire),
            "total_us": self.total_us.load(Ordering::Acquire),
            "exit_counts": {
                "total": self.exit_count_total.load(Ordering::Acquire),
                "pmio_out_fast": self.exit_count_pmio_out_fast.load(Ordering::Acquire),
                "pmio_in_fast": self.exit_count_pmio_in_fast.load(Ordering::Acquire),
                "pmio_slow": self.exit_count_pmio_slow.load(Ordering::Acquire),
                "interrupted": self.exit_count_interrupted.load(Ordering::Acquire),
                "halt": self.exit_count_halt.load(Ordering::Acquire),
                "mmio": self.exit_count_mmio.load(Ordering::Acquire),
                "intwin": self.exit_count_intwin.load(Ordering::Acquire),
            },
        })
        .to_string()
    }

    ///
    /// # Description
    ///
    /// Writes the timing data to host stderr as a single tagged line.
    ///
    /// The format is `PERF_TIMINGS:{...json...}\n` so that external tools can identify and
    /// parse the line.
    ///
    pub fn emit_to_stderr(&self) {
        eprintln!("{}{}", PERF_TIMINGS_PREFIX, self.to_json());
    }
}
