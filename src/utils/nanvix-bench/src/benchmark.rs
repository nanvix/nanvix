// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::env::get_proj_root;
use ::nanvix::hwloc::HwLoc;
use ::std::{
    fmt,
    process::Child,
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
    EchoBreakdown,
    EchoBreakdownL2,
    RoundTripLatency,
    WarmStart,
    WarmStartL2,
    WarmStartVMM,
}

impl BenchmarkFlavour {
    pub fn get_program(&self) -> String {
        match self {
            BenchmarkFlavour::BootTime => format!("{}/bin/noop-rust-nostd.elf", get_proj_root()),
            BenchmarkFlavour::ColdStart => format!("{}/bin/echo-rust-nostd.elf", get_proj_root()),
            BenchmarkFlavour::ColdStartL2 => format!("{}/bin/echo-rust-nostd.elf", get_proj_root()),
            BenchmarkFlavour::EchoBreakdown | BenchmarkFlavour::EchoBreakdownL2 => {
                format!("{}/bin/echo-rust-nostd.elf", get_proj_root())
            },
            BenchmarkFlavour::RoundTripLatency => {
                format!("{}/bin/echo-rust-nostd.elf", get_proj_root())
            },
            BenchmarkFlavour::WarmStart => format!("{}/bin/echo-rust-nostd.elf", get_proj_root()),
            BenchmarkFlavour::WarmStartL2 => format!("{}/bin/echo-rust-nostd.elf", get_proj_root()),
            BenchmarkFlavour::WarmStartVMM => {
                format!("{}/bin/echo-rust-nostd.elf", get_proj_root())
            },
        }
    }
}

impl fmt::Display for BenchmarkFlavour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BenchmarkFlavour::BootTime => "boot-time",
            BenchmarkFlavour::ColdStart => "cold-start",
            BenchmarkFlavour::ColdStartL2 => "cold-start-l2",
            BenchmarkFlavour::EchoBreakdown => "echo-breakdown",
            BenchmarkFlavour::EchoBreakdownL2 => "echo-breakdown-l2",
            BenchmarkFlavour::RoundTripLatency => "round-trip-latency",
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
            "echo-breakdown" => Ok(BenchmarkFlavour::EchoBreakdown),
            "echo-breakdown-l2" => Ok(BenchmarkFlavour::EchoBreakdownL2),
            "round-trip-latency" => Ok(BenchmarkFlavour::RoundTripLatency),
            "warm-start" => Ok(BenchmarkFlavour::WarmStart),
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
    pub nanvixd: Option<Child>,
    pub nanvixd_client: reqwest::Client,
    pub nanvixd_tmp_dir: String,
    pub nanvixd_toolchain_bin_dir: String,
    pub user_vm_id: Option<String>,
}
