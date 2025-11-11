// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::benchmark::BenchmarkFlavour;
use anyhow::Result;
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
    num_concurrent_vms: Option<usize>,
    toolchain_bin_dir: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    const OPT_HELP: &'static str = "-help";
    const OPT_BENCHMARK: &'static str = "-benchmark";
    const OPT_HWLOC: &'static str = "-hwloc";
    const OPT_ITERATIONS: &'static str = "-iterations";
    const OPT_NUM_CONCURRENT_VMS: &'static str = "-num-concurrent-vms";
    const OPT_TOOLCHAIN_BIN_DIR: &'static str = "-toolchain-bin-dir";

    fn usage(program_name: &str) {
        println!(
            "\
Nanvix Benchmarks - Benchmarking suite for Nanvix OS performance.

Usage:
  {program_name} {benchmark} [OPTIONS]

Benchmarks:
  boot-time              Measure raw user VM boot latency.
  cold-start             Measure start-up latency from client's perspective.
  cold-start-l2          Same as cold-start, but deploy linuxd insdie an L2 VM.
  cold-start-uvm         Measure start-up latency of the user VM only, excluding linuxd.
  concurrent             Measure cold-start times as we increase the number of concurrent user VMs.
  concurrent-l2          Same as concurrent, but deploy linuxd inside an L2 VM.
  echo-breakdown         Analyze the latency contributions of each step in the data path.
  echo-breakdown-l2      Same as echo-breakdown, but deploy linuxd inside L2 VM.
  round-trip-latency     Measure latency (warm-start) as we increase the payload size.
  warm-start             Measure round-trip latency from client's perspective.
  warm-start-l2          Same as warm-start, but deploy linuxd inside an L2 VM.
  warm-start-vmm         Measure raw round-trip latency inside the user VM.

Options:
  {benchmark} <benchmark>             Select which benchmark to run (required).
  {hwloc} <hwloc.json>                Hardware locality configuration file for CPU \
             affinity/topology.
  {iterations} <num>                  Number of iterations to run (default: 100).
  {num_concurrent_vms} <num>          Number of concurrent VMs to run (mandatory for concurrent \
             benchmarks).
  {toolchain_bin_dir} <toolchain_dir> Directory containing toolchain binaries (cloud-hypervisor, \
             etc.).
  {help}                              Show this help message and exit.

Examples:
  # Run the cold-start benchmark
  {program_name} {benchmark} cold-start

  # Run boot-time with a custom hwloc and 1000 iterations
  {program_name} {benchmark} boot-time {hwloc} hwloc.json {iterations} 1000

  # Run concurrent benchmark with 4 concurrent VMs
  {program_name} {benchmark} concurrent {num_concurrent_vms} 4
",
            program_name = program_name,
            benchmark = Self::OPT_BENCHMARK,
            hwloc = Self::OPT_HWLOC,
            iterations = Self::OPT_ITERATIONS,
            num_concurrent_vms = Self::OPT_NUM_CONCURRENT_VMS,
            toolchain_bin_dir = Self::OPT_TOOLCHAIN_BIN_DIR,
            help = Self::OPT_HELP,
        );
    }

    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut benchmark_str: String = String::new();
        let mut hwloc_file: Option<String> = None;
        let mut iterations: usize = 100;
        let mut num_concurrent_vms: Option<usize> = None;
        let mut toolchain_bin_dir: String = "./toolchain/bin".to_string();

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    process::exit(0);
                },
                Self::OPT_BENCHMARK => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_BENCHMARK));
                    }
                    benchmark_str = args[i].clone();
                },
                Self::OPT_HWLOC => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_HWLOC));
                    }

                    hwloc_file = Some(args[i].clone());
                },
                Self::OPT_ITERATIONS => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!("missing value for: {}", Self::OPT_ITERATIONS));
                    }
                    iterations = args[i].parse::<usize>()?;
                },
                Self::OPT_NUM_CONCURRENT_VMS => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_NUM_CONCURRENT_VMS
                        ));
                    }
                    num_concurrent_vms = Some(args[i].parse::<usize>()?);
                },
                Self::OPT_TOOLCHAIN_BIN_DIR => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_TOOLCHAIN_BIN_DIR
                        ));
                    }
                    toolchain_bin_dir = args[i].clone();
                },
                arg => {
                    Self::usage(args[0].as_str());
                    return Err(anyhow::anyhow!("invalid argument: {arg}"));
                },
            }

            i += 1;
        }

        match BenchmarkFlavour::from_str(benchmark_str.as_str()) {
            Ok(benchmark) => {
                match benchmark {
                    // The concurrent benchmarks take slightly different command-line arguments.
                    BenchmarkFlavour::Concurrent | BenchmarkFlavour::ConcurrentL2 => {
                        // Must pass -num-concurrent-vms
                        if num_concurrent_vms.is_none() {
                            Self::usage(args[0].as_str());
                            return Err(anyhow::anyhow!(
                                "missing value for: {}",
                                Self::OPT_NUM_CONCURRENT_VMS
                            ));
                        }

                        // Must not pass -hwloc.
                        if hwloc_file.is_some() {
                            Self::usage(args[0].as_str());
                            return Err(anyhow::anyhow!(
                                "{benchmark} benchmark does not take {} flag",
                                Self::OPT_HWLOC,
                            ));
                        }
                    },
                    _ => {
                        if num_concurrent_vms.is_some() {
                            Self::usage(args[0].as_str());
                            return Err(anyhow::anyhow!(
                                "unsupported argument for this benchmark: {}",
                                Self::OPT_NUM_CONCURRENT_VMS
                            ));
                        }
                    },
                }

                Ok(Self {
                    benchmark,
                    hwloc_file,
                    iterations,
                    num_concurrent_vms,
                    toolchain_bin_dir,
                })
            },
            Err(_) => {
                Self::usage(args[0].as_str());
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

    pub fn num_concurrent_vms(&self) -> Option<usize> {
        self.num_concurrent_vms
    }

    pub fn toolchain_bin_dir(&self) -> String {
        self.toolchain_bin_dir.clone()
    }
}
