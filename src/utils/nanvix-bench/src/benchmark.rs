// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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
    SnapshotRestore,
    VfsBench,
    WarmStartGateway,
    WarmStartVMM,
    WarmStartSocket,
}

impl BenchmarkFlavour {
    /// Returns `true` when this benchmark needs an initrd containing the guest daemon stack.
    pub fn needs_standalone_image(&self) -> bool {
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
        let ext: &str = if self.needs_standalone_image() {
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
            BenchmarkFlavour::SnapshotRestore => "snapshot-restore",
            BenchmarkFlavour::VfsBench => "vfs-bench",
            BenchmarkFlavour::WarmStartGateway => "warm-start-gateway",
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
            "snapshot-restore" => Ok(BenchmarkFlavour::SnapshotRestore),
            "vfs-bench" => Ok(BenchmarkFlavour::VfsBench),
            "warm-start-gateway" => Ok(BenchmarkFlavour::WarmStartGateway),
            "warm-start-vmm" => Ok(BenchmarkFlavour::WarmStartVMM),
            "warm-start-socket" => Ok(BenchmarkFlavour::WarmStartSocket),
            _ => Err(format!("Invalid benchmark type: {}", s)),
        }
    }
}

pub struct Benchmark {
    pub iterations: usize,
    pub payload_size_override: Option<usize>,
    pub flavour: BenchmarkFlavour,
    pub workspace_root: PathBuf,
}
