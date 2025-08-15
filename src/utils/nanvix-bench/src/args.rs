// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::benchmark::BenchmarkFlavour;
use anyhow::Result;
use log::error;
use std::{
    process,
    str::FromStr,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct Args {
    benchmark: BenchmarkFlavour,
    hwloc_file: Option<String>,
    iterations: usize,
    tmp_dir: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    const OPT_HELP: &'static str = "-help";
    const OPT_BENCHMARK: &'static str = "-benchmark";
    const OPT_HWLOC: &'static str = "-hwloc";
    const OPT_ITERATIONS: &'static str = "-iterations";
    const OPT_TMP_DIR: &'static str = "-tmp-dir";

    fn usage() -> String {
        format!(
            "usage: ./bin/nanvix-bench.elf {} \
             [boot-time,cold-start,warm-start,warm-start-vmm,echo-breakdown] [{} \
             <path_to_hwloc.json> {} <iterations> {} <tmp_dir>]",
            Self::OPT_BENCHMARK,
            Self::OPT_HWLOC,
            Self::OPT_ITERATIONS,
            Self::OPT_TMP_DIR,
        )
    }

    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut benchmark_str: String = String::new();
        let mut hwloc_file: Option<String> = None;
        let mut iterations: usize = 100;
        let mut tmp_dir: String = "/tmp/".to_string();

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

                    hwloc_file = Some(args[i].clone());
                },
                Self::OPT_ITERATIONS => {
                    i += 1;
                    if i >= args.len() {
                        error!("{}", Self::usage());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_ITERATIONS));
                    }
                    iterations = args[i].parse::<usize>()?;
                },
                Self::OPT_TMP_DIR => {
                    i += 1;
                    if i >= args.len() {
                        error!("{}", Self::usage());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_TMP_DIR));
                    }
                    tmp_dir = args[i].clone();
                },
                arg => {
                    error!("{}", Self::usage());
                    return Err(anyhow::anyhow!("invalid argument: {arg}"));
                },
            }

            i += 1;
        }

        match BenchmarkFlavour::from_str(benchmark_str.as_str()) {
            Ok(benchmark) => Ok(Self {
                benchmark,
                hwloc_file,
                iterations,
                tmp_dir,
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

    pub fn hwloc_file(&self) -> Option<String> {
        self.hwloc_file.clone()
    }

    pub fn iterations(&self) -> usize {
        self.iterations
    }

    pub fn tmp_dir(&self) -> String {
        self.tmp_dir.clone()
    }
}
