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
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    const OPT_HELP: &'static str = "-help";
    const OPT_BENCHMARK: &'static str = "-benchmark";

    fn usage() -> String {
        format!(
            "usage: ./bin/nanvix-bench.elf {} [cold-start,warm-start,echo-breakdown]",
            Self::OPT_BENCHMARK
        )
    }

    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut benchmark_str: String = String::new();

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
                _ => {
                    error!("{}", Self::usage());
                    return Err(anyhow::anyhow!("invalid argument"));
                },
            }

            i += 1;
        }

        match BenchmarkFlavour::from_str(benchmark_str.as_str()) {
            Ok(benchmark) => Ok(Self { benchmark }),
            Err(_) => {
                error!("{}", Self::usage());
                Err(anyhow::anyhow!("invalid argument"))
            },
        }
    }

    pub fn benchmark(&self) -> BenchmarkFlavour {
        self.benchmark.clone()
    }
}
