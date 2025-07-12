// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hwloc::HwLoc;
use std::{
    fmt,
    net::TcpStream,
    process::Child,
    str::FromStr,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone)]
pub enum BenchmarkFlavour {
    ColdStart,
    WarmStart,
    WarmStartVMM,
    EchoBreakdown,
}

impl fmt::Display for BenchmarkFlavour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
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
    pub hwloc: Option<HwLoc>,
    pub flavour: BenchmarkFlavour,
    pub gateway_address: String,
    pub linuxd_address: String,
    pub linuxd: Option<Child>,
    pub nanovm: Option<Child>,
    pub gateway: Option<TcpStream>,
}
