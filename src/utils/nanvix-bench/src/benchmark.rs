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
    ColdStartUvm,
    Concurrent,
    EchoBreakdown,
    RoundTripLatency,
    SnapshotRestore,
    VfsBench,
    WarmStart,
    WarmStartVMM,
    WarmStartSocket,
}

impl BenchmarkFlavour {
    /// Returns `true` when this benchmark requires the `timestamp-messages` feature.
    pub fn requires_timestamp_messages(&self) -> bool {
        matches!(self, BenchmarkFlavour::EchoBreakdown)
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
            BenchmarkFlavour::WarmStartSocket => {
                format!("{}/bin/socket-echo-rust-nostd.{ext}", root.display())
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
            BenchmarkFlavour::ColdStartUvm => "cold-start-uvm",
            BenchmarkFlavour::Concurrent => "concurrent",
            BenchmarkFlavour::EchoBreakdown => "echo-breakdown",
            BenchmarkFlavour::RoundTripLatency => "round-trip-latency",
            BenchmarkFlavour::SnapshotRestore => "snapshot-restore",
            BenchmarkFlavour::VfsBench => "vfs-bench",
            BenchmarkFlavour::WarmStart => "warm-start",
            BenchmarkFlavour::WarmStartVMM => "warm-start-vmm",
            BenchmarkFlavour::WarmStartSocket => "warm-start-socket",
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
            "cold-start-uvm" => Ok(BenchmarkFlavour::ColdStartUvm),
            "concurrent" => Ok(BenchmarkFlavour::Concurrent),
            "echo-breakdown" => Ok(BenchmarkFlavour::EchoBreakdown),
            "round-trip-latency" => Ok(BenchmarkFlavour::RoundTripLatency),
            "snapshot-restore" => Ok(BenchmarkFlavour::SnapshotRestore),
            "vfs-bench" => Ok(BenchmarkFlavour::VfsBench),
            "warm-start" => Ok(BenchmarkFlavour::WarmStart),
            "warm-start-vmm" => Ok(BenchmarkFlavour::WarmStartVMM),
            "warm-start-socket" => Ok(BenchmarkFlavour::WarmStartSocket),
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
    pub nanvixd_tmp_dir: String,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    pub user_vm_id: Option<String>,
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
