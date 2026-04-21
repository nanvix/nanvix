// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::benchmark::BenchmarkFlavour;
use ::anyhow::Result;
use ::nanvixd::config::DEFAULT_TMP_DIRECTORY;
use ::std::str::FromStr;

//==================================================================================================
// Structures
//==================================================================================================

pub struct Args {
    benchmark: BenchmarkFlavour,
    hwloc_file: Option<String>,
    iterations: usize,
    num_concurrent_vms: Option<usize>,
    netns_pool_size: Option<usize>,
    clh_bin_path: String,
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
    const OPT_NUM_CONCURRENT_VMS: &'static str = "-num-concurrent-vms";
    const OPT_NETNS_POOL_SIZE: &'static str = "-netns-pool-size";
    const DEFAULT_NETNS_POOL_SIZE: usize = ::nanvixd::args::Args::DEFAULT_NETNS_POOL_SIZE;
    const OPT_CLH_BIN_PATH: &'static str = "-clh-bin-path";
    const OPT_TMP_DIR: &'static str = "-tmp-dir";

    fn usage(program_name: &str) {
        let mut benchmarks = String::new();

        // VMM-level benchmarks are always available.
        benchmarks.push_str(
            "\
  boot-time              Measure raw user VM boot latency.\n",
        );

        // System-level benchmarks require multi-process, single-process, or standalone.
        if cfg!(any(feature = "multi-process", feature = "single-process", feature = "standalone"))
        {
            benchmarks.push_str(
                "\
  cold-start             Measure start-up latency from client's perspective.\n",
            );
        }

        if cfg!(any(feature = "multi-process", feature = "single-process")) {
            benchmarks.push_str(
                "\
  cold-start-uvm         Measure start-up latency of the user VM only, excluding linuxd.
  round-trip-latency     Measure latency (warm-start) as we increase the payload size.\n",
            );
        }

        // echo-breakdown requires timestamp-messages in addition to multi/single-process.
        if cfg!(feature = "timestamp-messages") {
            benchmarks.push_str(
                "\
  echo-breakdown         Analyze the latency contributions of each step in the data path.\n",
            );
        }

        // concurrent requires multi-process specifically.
        if cfg!(feature = "multi-process") {
            benchmarks.push_str(
                "\
  concurrent             Measure cold-start times as we increase the number of concurrent user \
                 VMs.\n",
            );
        }

        // L2 variants.
        if cfg!(feature = "l2") {
            benchmarks.push_str(
                "\
  cold-start-l2          Same as cold-start, but deploy linuxd inside an L2 VM.
  concurrent-l2          Same as concurrent, but deploy linuxd inside an L2 VM.
  echo-breakdown-l2      Same as echo-breakdown, but deploy linuxd inside L2 VM.
  warm-start-l2          Same as warm-start, but deploy linuxd inside an L2 VM.\n",
            );
        }

        benchmarks.push_str(
            "\
  snapshot-restore       Measure snapshot restore latency vs boot-time.\n",
        );

        if cfg!(feature = "standalone") {
            benchmarks.push_str(
                "\
  vfs-bench              Measure VFS operation latencies on a dense ramfs (standalone).\n",
            );
        }

        if cfg!(any(feature = "multi-process", feature = "single-process")) {
            benchmarks.push_str(
                "\
  warm-start             Measure round-trip latency from client's perspective.\n",
            );
        }

        benchmarks.push_str(
            "\
  warm-start-vmm         Measure raw round-trip latency inside the user VM.",
        );

        println!(
            "\
Nanvix Benchmarks - Benchmarking suite for Nanvix OS performance.

Usage:
  {program_name} {benchmark} [OPTIONS]

Benchmarks:
{benchmarks}

Options:
  {benchmark} <benchmark>             Select which benchmark to run (required).
  {hwloc} <hwloc.json>                Hardware locality configuration file for CPU \
             affinity/topology.
  {iterations} <num>                  Number of iterations to run (default: 100).
  {num_concurrent_vms} <num>          Number of concurrent VMs (mandatory for concurrent and \
             concurrent-l2 benchmarks).
  {netns_pool_size} <size>            Netns pool prefill size for nanvixd (concurrent-l2 only; \
             default: {default_netns_pool_size}). Other L2 benchmarks use 1; non-L2 benchmarks \
             ignore this flag.
  {clh_bin_path} <clh_bin_path>       Path to the cloud-hypervisor binary directory.
  {tmp_dir} <tmp_dir>                Base directory for temporary files (default: \
             {DEFAULT_TMP_DIRECTORY}).
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
            netns_pool_size = Self::OPT_NETNS_POOL_SIZE,
            default_netns_pool_size = Self::DEFAULT_NETNS_POOL_SIZE,
            clh_bin_path = Self::OPT_CLH_BIN_PATH,
            tmp_dir = Self::OPT_TMP_DIR,
            DEFAULT_TMP_DIRECTORY = DEFAULT_TMP_DIRECTORY,
            help = Self::OPT_HELP,
        );
    }

    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut benchmark_str: String = String::new();
        let mut hwloc_file: Option<String> = None;
        let mut iterations: usize = 100;
        let mut num_concurrent_vms: Option<usize> = None;
        let mut netns_pool_size: Option<usize> = None;
        let mut clh_bin_path: String = "./toolchain/bin".to_string();
        let mut tmp_dir: String = DEFAULT_TMP_DIRECTORY.to_string();

        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                Self::OPT_HELP => {
                    Self::usage(args[0].as_str());
                    ::std::process::exit(0);
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
                Self::OPT_NETNS_POOL_SIZE => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_NETNS_POOL_SIZE
                        ));
                    }
                    netns_pool_size = Some(args[i].parse::<usize>()?);
                },
                Self::OPT_CLH_BIN_PATH => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_CLH_BIN_PATH
                        ));
                    }
                    clh_bin_path = args[i].clone();
                },
                Self::OPT_TMP_DIR => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(::anyhow::anyhow!("missing value for: {}", Self::OPT_TMP_DIR));
                    }
                    tmp_dir = args[i].clone();
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

                // Reject -netns-pool-size when it would be silently ignored.
                if netns_pool_size.is_some() {
                    match benchmark {
                        BenchmarkFlavour::ConcurrentL2 => {},
                        _ => {
                            Self::usage(args[0].as_str());
                            return Err(anyhow::anyhow!(
                                "{benchmark} benchmark does not accept {}",
                                Self::OPT_NETNS_POOL_SIZE,
                            ));
                        },
                    }
                }

                // Derive netns pool size from the benchmark flavour.
                let netns_pool_size = match benchmark {
                    BenchmarkFlavour::ConcurrentL2 => {
                        Some(netns_pool_size.unwrap_or(Self::DEFAULT_NETNS_POOL_SIZE))
                    },
                    _ if benchmark.is_l2() => Some(1),
                    _ => None,
                };

                Ok(Self {
                    benchmark,
                    hwloc_file,
                    iterations,
                    num_concurrent_vms,
                    netns_pool_size,
                    clh_bin_path,
                    tmp_dir,
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

    pub fn clh_bin_path(&self) -> String {
        self.clh_bin_path.clone()
    }

    pub fn netns_pool_size(&self) -> Option<usize> {
        self.netns_pool_size
    }

    pub fn tmp_dir(&self) -> String {
        self.tmp_dir.clone()
    }
}
