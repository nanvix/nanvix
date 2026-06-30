// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nanvix::hwloc::HwLoc;
#[cfg(any(feature = "multi-process", feature = "single-process"))]
use ::std::process::Child;
use ::std::{
    fmt,
    path::{
        Path,
        PathBuf,
    },
    str::FromStr,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone)]
pub enum BenchmarkFlavour {
    BootTime,
    ColdStart,
    ColdStartL2,
    ColdStartUvm,
    Concurrent,
    ConcurrentL2,
    EchoBreakdown,
    EchoBreakdownL2,
    RoundTripLatency,
    SnapshotRestore,
    VfsBench,
    WarmStart,
    WarmStartL2,
    WarmStartVMM,
}

impl BenchmarkFlavour {
    /// Returns `true` when linuxd is deployed inside an L2 VM for this benchmark.
    pub fn is_l2(&self) -> bool {
        matches!(
            self,
            BenchmarkFlavour::ColdStartL2
                | BenchmarkFlavour::ConcurrentL2
                | BenchmarkFlavour::EchoBreakdownL2
                | BenchmarkFlavour::WarmStartL2
        )
    }

    /// Returns the linuxd deployment mode for this benchmark.
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    pub fn deployment(&self) -> LinuxdDeployment {
        if self.is_l2() {
            LinuxdDeployment::L2Vm
        } else {
            LinuxdDeployment::Process
        }
    }

    /// Returns `true` when this benchmark requires the `timestamp-messages` feature.
    pub fn requires_timestamp_messages(&self) -> bool {
        matches!(self, BenchmarkFlavour::EchoBreakdown | BenchmarkFlavour::EchoBreakdownL2)
    }

    /// Returns `true` when this benchmark needs nanvixd (system-level benchmark).
    pub fn needs_nanvixd(&self) -> bool {
        !matches!(
            self,
            BenchmarkFlavour::BootTime
                | BenchmarkFlavour::SnapshotRestore
                | BenchmarkFlavour::WarmStartVMM
        )
    }

    pub fn get_program(&self, root: &Path) -> String {
        // VMM benchmarks (boot-time, snapshot-restore, warm-start-vmm) spawn a VM directly
        // without nanvixd, so they always use the bare .elf binary. System benchmarks that need
        // the daemon stack use .initrd in standalone mode to bundle procd, memd, vfsd, and the
        // application binary.
        let ext: &str = if cfg!(feature = "standalone") && self.needs_nanvixd() {
            "initrd"
        } else {
            "elf"
        };
        match self {
            BenchmarkFlavour::BootTime => {
                format!("{}/bin/noop-rust-nostd.{ext}", root.display())
            },
            BenchmarkFlavour::SnapshotRestore => {
                format!("{}/bin/snapshot-rust-nostd.{ext}", root.display())
            },
            BenchmarkFlavour::VfsBench => {
                format!("{}/bin/vfs-bench-nostd.{ext}", root.display())
            },
            _ => {
                format!("{}/bin/echo-rust-nostd.{ext}", root.display())
            },
        }
    }

    /// Returns the ramfs image path for this benchmark, if one is required.
    pub fn get_ramfs(&self, root: &Path) -> Option<String> {
        match self {
            BenchmarkFlavour::VfsBench => {
                Some(format!("{}/bin/{}", root.display(), vfs_bench_common::VFS_BENCH_IMG))
            },
            _ => None,
        }
    }
}

impl fmt::Display for BenchmarkFlavour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BenchmarkFlavour::BootTime => "boot-time",
            BenchmarkFlavour::ColdStart => "cold-start",
            BenchmarkFlavour::ColdStartL2 => "cold-start-l2",
            BenchmarkFlavour::ColdStartUvm => "cold-start-uvm",
            BenchmarkFlavour::Concurrent => "concurrent",
            BenchmarkFlavour::ConcurrentL2 => "concurrent-l2",
            BenchmarkFlavour::EchoBreakdown => "echo-breakdown",
            BenchmarkFlavour::EchoBreakdownL2 => "echo-breakdown-l2",
            BenchmarkFlavour::RoundTripLatency => "round-trip-latency",
            BenchmarkFlavour::SnapshotRestore => "snapshot-restore",
            BenchmarkFlavour::VfsBench => "vfs-bench",
            BenchmarkFlavour::WarmStart => "warm-start",
            BenchmarkFlavour::WarmStartL2 => "warm-start-l2",
            BenchmarkFlavour::WarmStartVMM => "warm-start-vmm",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for BenchmarkFlavour {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "boot-time" => Ok(BenchmarkFlavour::BootTime),
            "cold-start" => Ok(BenchmarkFlavour::ColdStart),
            "cold-start-l2" => Ok(BenchmarkFlavour::ColdStartL2),
            "cold-start-uvm" => Ok(BenchmarkFlavour::ColdStartUvm),
            "concurrent" => Ok(BenchmarkFlavour::Concurrent),
            "concurrent-l2" => Ok(BenchmarkFlavour::ConcurrentL2),
            "echo-breakdown" => Ok(BenchmarkFlavour::EchoBreakdown),
            "echo-breakdown-l2" => Ok(BenchmarkFlavour::EchoBreakdownL2),
            "round-trip-latency" => Ok(BenchmarkFlavour::RoundTripLatency),
            "snapshot-restore" => Ok(BenchmarkFlavour::SnapshotRestore),
            "vfs-bench" => Ok(BenchmarkFlavour::VfsBench),
            "warm-start" => Ok(BenchmarkFlavour::WarmStart),
            "warm-start-l2" => Ok(BenchmarkFlavour::WarmStartL2),
            "warm-start-vmm" => Ok(BenchmarkFlavour::WarmStartVMM),
            _ => Err(format!("Invalid benchmark type: {}", s)),
        }
    }
}

pub struct Benchmark {
    pub iterations: usize,
    pub payload_size: usize,
    pub payload_size_override: Option<usize>,
    pub hwloc_file: Option<String>,
    pub hwloc: Option<HwLoc>,
    pub flavour: BenchmarkFlavour,
    pub workspace_root: PathBuf,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    pub nanvixd: Option<Child>,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    pub nanvixd_client: reqwest::Client,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    pub nanvixd_clh_bin_path: String,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    pub nanvixd_netns_pool_size: Option<usize>,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    pub nanvixd_tmp_dir: String,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    pub user_vm_id: Option<String>,
}

///
/// # Description
///
/// Linuxd deployment mode.
///
#[cfg(any(feature = "multi-process", feature = "single-process"))]
#[derive(Clone, Copy, PartialEq)]
pub enum LinuxdDeployment {
    /// Linuxd deployed inside an L2 VM.
    L2Vm,
    /// Linuxd deployed as a userspace process.
    Process,
}

///
/// # Description
///
/// User VM deployment mode.
///
#[cfg(any(feature = "multi-process", feature = "single-process"))]
#[derive(PartialEq)]
pub enum UserVmDeployment {
    /// Ensure each user VM gets a different linuxd instance.
    OneToOne,
    /// Start a pre-warm user VM with the same configuration and kill it.
    PreWarm,
}
