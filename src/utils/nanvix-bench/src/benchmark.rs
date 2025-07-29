// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::env::get_proj_root;
use hwloc::HwLoc;
use std::{
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
    WarmStart,
    WarmStartVMM,
    EchoBreakdown,
}

impl BenchmarkFlavour {
    pub fn get_program(&self) -> String {
        match self {
            BenchmarkFlavour::BootTime => format!("{}/bin/noop-rust-nostd.elf", get_proj_root()),
            BenchmarkFlavour::ColdStart => format!("{}/bin/echo-rust-nostd.elf", get_proj_root()),
            BenchmarkFlavour::WarmStart => format!("{}/bin/echo-rust-nostd.elf", get_proj_root()),
            BenchmarkFlavour::WarmStartVMM => format!("{}/bin/echo-rust-nostd.elf", get_proj_root()),
            BenchmarkFlavour::EchoBreakdown => format!("{}/bin/echo-rust-nostd.elf", get_proj_root()),
        }
    }
}

impl fmt::Display for BenchmarkFlavour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BenchmarkFlavour::BootTime => "boot-time",
            BenchmarkFlavour::ColdStart => "cold-start",
            BenchmarkFlavour::WarmStart => "warm-start",
            BenchmarkFlavour::WarmStartVMM => "warm-start-vmm",
            BenchmarkFlavour::EchoBreakdown => "echo-breakdown",
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
            "warm-start" => Ok(BenchmarkFlavour::WarmStart),
            "warm-start-vmm" => Ok(BenchmarkFlavour::WarmStartVMM),
            "echo-breakdown" => Ok(BenchmarkFlavour::EchoBreakdown),
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
    pub user_vm_id: Option<String>,
}
