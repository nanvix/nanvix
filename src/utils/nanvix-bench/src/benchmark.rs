// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nanvix::hwloc::HwLoc;
use ::std::{
    fmt,
    path::{
        Path,
        PathBuf,
    },
    process::Child,
    str::FromStr,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone)]
pub enum BenchmarkFlavour {
    BootTime,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    ColdStart,
    #[cfg(feature = "l2")]
    ColdStartL2,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    ColdStartUvm,
    #[cfg(feature = "multi-process")]
    Concurrent,
    #[cfg(feature = "l2")]
    ConcurrentL2,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    EchoBreakdown,
    #[cfg(feature = "l2")]
    EchoBreakdownL2,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    RoundTripLatency,
    SnapshotRestore,
    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    WarmStart,
    #[cfg(feature = "l2")]
    WarmStartL2,
    WarmStartVMM,
}

impl BenchmarkFlavour {
    /// Returns `true` when linuxd is deployed inside an L2 VM for this benchmark.
    pub fn is_l2(&self) -> bool {
        #[cfg(feature = "l2")]
        {
            matches!(
                self,
                BenchmarkFlavour::ColdStartL2
                    | BenchmarkFlavour::ConcurrentL2
                    | BenchmarkFlavour::EchoBreakdownL2
                    | BenchmarkFlavour::WarmStartL2
            )
        }
        #[cfg(not(feature = "l2"))]
        {
            false
        }
    }

    pub fn get_program(&self, root: &Path) -> String {
        match self {
            BenchmarkFlavour::BootTime => {
                format!("{}/bin/noop-rust-nostd.elf", root.display())
            },
            BenchmarkFlavour::SnapshotRestore => {
                format!("{}/bin/snapshot-rust-nostd.elf", root.display())
            },
            BenchmarkFlavour::WarmStartVMM => {
                format!("{}/bin/echo-rust-nostd.elf", root.display())
            },
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            BenchmarkFlavour::ColdStart
            | BenchmarkFlavour::ColdStartUvm
            | BenchmarkFlavour::EchoBreakdown
            | BenchmarkFlavour::RoundTripLatency
            | BenchmarkFlavour::WarmStart => {
                format!("{}/bin/echo-rust-nostd.elf", root.display())
            },
            #[cfg(feature = "multi-process")]
            BenchmarkFlavour::Concurrent => {
                format!("{}/bin/echo-rust-nostd.elf", root.display())
            },
            #[cfg(feature = "l2")]
            BenchmarkFlavour::ColdStartL2
            | BenchmarkFlavour::ConcurrentL2
            | BenchmarkFlavour::EchoBreakdownL2
            | BenchmarkFlavour::WarmStartL2 => {
                format!("{}/bin/echo-rust-nostd.elf", root.display())
            },
        }
    }
}

impl fmt::Display for BenchmarkFlavour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BenchmarkFlavour::BootTime => "boot-time",
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            BenchmarkFlavour::ColdStart => "cold-start",
            #[cfg(feature = "l2")]
            BenchmarkFlavour::ColdStartL2 => "cold-start-l2",
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            BenchmarkFlavour::ColdStartUvm => "cold-start-uvm",
            #[cfg(feature = "multi-process")]
            BenchmarkFlavour::Concurrent => "concurrent",
            #[cfg(feature = "l2")]
            BenchmarkFlavour::ConcurrentL2 => "concurrent-l2",
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            BenchmarkFlavour::EchoBreakdown => "echo-breakdown",
            #[cfg(feature = "l2")]
            BenchmarkFlavour::EchoBreakdownL2 => "echo-breakdown-l2",
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            BenchmarkFlavour::RoundTripLatency => "round-trip-latency",
            BenchmarkFlavour::SnapshotRestore => "snapshot-restore",
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            BenchmarkFlavour::WarmStart => "warm-start",
            #[cfg(feature = "l2")]
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
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            "cold-start" => Ok(BenchmarkFlavour::ColdStart),
            #[cfg(feature = "l2")]
            "cold-start-l2" => Ok(BenchmarkFlavour::ColdStartL2),
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            "cold-start-uvm" => Ok(BenchmarkFlavour::ColdStartUvm),
            #[cfg(feature = "multi-process")]
            "concurrent" => Ok(BenchmarkFlavour::Concurrent),
            #[cfg(feature = "l2")]
            "concurrent-l2" => Ok(BenchmarkFlavour::ConcurrentL2),
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            "echo-breakdown" => Ok(BenchmarkFlavour::EchoBreakdown),
            #[cfg(feature = "l2")]
            "echo-breakdown-l2" => Ok(BenchmarkFlavour::EchoBreakdownL2),
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            "round-trip-latency" => Ok(BenchmarkFlavour::RoundTripLatency),
            "snapshot-restore" => Ok(BenchmarkFlavour::SnapshotRestore),
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            "warm-start" => Ok(BenchmarkFlavour::WarmStart),
            #[cfg(feature = "l2")]
            "warm-start-l2" => Ok(BenchmarkFlavour::WarmStartL2),
            "warm-start-vmm" => Ok(BenchmarkFlavour::WarmStartVMM),
            _ => Err(format!("Invalid benchmark type: {}", s)),
        }
    }
}

pub struct Benchmark {
    pub iterations: usize,
    pub hwloc_file: Option<String>,
    pub hwloc: Option<HwLoc>,
    pub flavour: BenchmarkFlavour,
    pub workspace_root: PathBuf,
    pub nanvixd: Option<Child>,
    pub nanvixd_client: reqwest::Client,
    pub nanvixd_toolchain_bin_dir: String,
    pub nanvixd_netns_pool_size: Option<usize>,
    pub nanvixd_tmp_dir: String,
    pub user_vm_id: Option<String>,
}

///
/// # Description
///
/// Linuxd deployment mode.
///
#[derive(PartialEq)]
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
#[derive(PartialEq)]
pub enum UserVmDeployment {
    /// Ensure each user VM gets a different linuxd instance.
    OneToOne,
    /// Start a pre-warm user VM with the same configuration and kill it.
    PreWarm,
}
