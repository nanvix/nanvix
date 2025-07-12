// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    benchmark::BenchmarkFlavour,
    hwloc::HwLoc,
};
use anyhow::Result;
use log::error;
use std::{
    fs::File,
    io::BufReader,
    process,
    str::FromStr,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct Args {
    benchmark: BenchmarkFlavour,
    hwloc: Option<HwLoc>,
    iterations: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    const OPT_HELP: &'static str = "-help";
    const OPT_BENCHMARK: &'static str = "-benchmark";
    const OPT_HWLOC: &'static str = "-hwloc";
    const OPT_ITERATIONS: &'static str = "-iterations";

    fn usage() -> String {
        format!(
            "usage: ./bin/nanvix-bench.elf {} [boot-time,cold-start,warm-start,echo-breakdown] \
             [{} <path_to_hwloc.json> {} <iterations>]",
            Self::OPT_BENCHMARK,
            Self::OPT_HWLOC,
            Self::OPT_ITERATIONS,
        )
    }

    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut benchmark_str: String = String::new();
        let mut hwloc: Option<HwLoc> = None;
        let mut iterations: usize = 100;

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    println!("{}", Self::usage());
                    process::exit(0);
                },
                Self::OPT_BENCHMARK => {
                    i += 1;
                    if i >= args.len() {
                        error!("{}", Self::usage());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_BENCHMARK));
                    }
                    benchmark_str = args[i].clone();
                },
                Self::OPT_HWLOC => {
                    i += 1;
                    if i >= args.len() {
                        error!("{}", Self::usage());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_HWLOC));
                    }

                    // Parse hwloc from JSON file.
                    let hwloc_file = File::open(args[i].clone())?;
                    let hwloc_reader = BufReader::new(hwloc_file);
                    hwloc = Some(serde_json::from_reader(hwloc_reader)?);
                },
                Self::OPT_ITERATIONS => {
                    i += 1;
                    if i >= args.len() {
                        error!("{}", Self::usage());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_ITERATIONS));
                    }
                    iterations = args[i].parse::<usize>()?;
                },
                _ => {
                    error!("{}", Self::usage());
                    return Err(anyhow::anyhow!("invalid argument"));
                },
            }

            i += 1;
        }

        match BenchmarkFlavour::from_str(benchmark_str.as_str()) {
            Ok(benchmark) => Ok(Self {
                benchmark,
                hwloc,
                iterations,
            }),
            Err(_) => {
                error!("{}", Self::usage());
                Err(anyhow::anyhow!("invalid argument"))
            },
        }
    }

    pub fn benchmark(&self) -> BenchmarkFlavour {
        self.benchmark.clone()
    }

    pub fn hwloc(&self) -> Option<HwLoc> {
        self.hwloc.clone()
    }

    pub fn iterations(&self) -> usize {
        self.iterations
    }
}
