// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    benchmark::BenchmarkFlavour,
    benchmarks::DEFAULT_PAYLOAD_SIZE,
};
use ::anyhow::Result;
#[cfg(feature = "single-process")]
use ::nanvixd::config::DEFAULT_TMP_DIRECTORY;
use ::std::str::FromStr;

//==================================================================================================
// Structures
//==================================================================================================

pub struct Args {
    benchmark: BenchmarkFlavour,
    hwloc_file: Option<String>,
    iterations: usize,
    #[cfg(feature = "single-process")]
    payload_size: usize,
    payload_size_override: Option<usize>,
    #[cfg(feature = "single-process")]
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
    const OPT_PAYLOAD_SIZE: &'static str = "-payload-size";
    const WARM_START_VMM_PAYLOAD_PREFIX_SIZE: usize = ::std::mem::size_of::<u32>();
    #[cfg(feature = "single-process")]
    const OPT_TMP_DIR: &'static str = "-tmp-dir";

    fn usage(program_name: &str) {
        let mut benchmarks = String::new();

        // VMM-level benchmarks are always available.
        benchmarks.push_str(
            "\
  boot-time              Measure raw user VM boot latency.\n",
        );

        // System-level benchmarks require single-process or standalone.
        if cfg!(any(feature = "single-process", feature = "standalone")) {
            benchmarks.push_str(
                "\
  cold-start             Measure start-up latency from client's perspective.\n",
            );
        }

        if cfg!(feature = "single-process") {
            benchmarks.push_str(
                "\
  cold-start-uvm         Measure start-up latency of the user VM only, excluding linuxd.
  round-trip-latency     Measure latency (warm-start) as we increase the payload size.\n",
            );
        }

        // echo-breakdown requires timestamp-messages in addition to single-process.
        if cfg!(feature = "timestamp-messages") {
            benchmarks.push_str(
                "\
  echo-breakdown         Analyze the latency contributions of each step in the data path.\n",
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

        if cfg!(feature = "single-process") {
            benchmarks.push_str(
                "\
  warm-start             Measure round-trip latency from client's perspective.\n",
            );
        }

        benchmarks.push_str(
            "\
  warm-start-vmm         Measure raw round-trip latency inside the user VM.",
        );

        if cfg!(feature = "standalone") {
            benchmarks.push_str(
                "\n  warm-start-socket      Measure round-trip latency over a guest TCP echo \
                 socket.",
            );
        }

        #[cfg(feature = "single-process")]
        let tmp_dir_option: String = format!(
            "  {} <tmp_dir>                Base directory for temporary files (default: {}).\n",
            Self::OPT_TMP_DIR,
            DEFAULT_TMP_DIRECTORY,
        );
        #[cfg(not(feature = "single-process"))]
        let tmp_dir_option: String = String::new();

        let help_option: String = format!(
            "  {}                              Show this help message and exit.\n",
            Self::OPT_HELP,
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
  {payload_size} <bytes>              Echo payload size for warm-start, warm-start-vmm, and \
             warm-start-socket benchmarks (default: {default_payload_size} for warm-start; \
             warm-start-vmm counts its {warm_start_vmm_prefix_size}-byte prefix; warm-start-vmm \
             and warm-start-socket sweep a range of sizes when omitted).
{tmp_dir_option}{help_option}

Examples:
  # Run the cold-start benchmark
  {program_name} {benchmark} cold-start

  # Run boot-time with a custom hwloc and 1000 iterations
  {program_name} {benchmark} boot-time {hwloc} hwloc.json {iterations} 1000

  # Run warm-start-vmm with a 32KiB payload
  {program_name} {benchmark} warm-start-vmm {payload_size} 32768
",
            program_name = program_name,
            benchmark = Self::OPT_BENCHMARK,
            hwloc = Self::OPT_HWLOC,
            iterations = Self::OPT_ITERATIONS,
            payload_size = Self::OPT_PAYLOAD_SIZE,
            default_payload_size = DEFAULT_PAYLOAD_SIZE,
            warm_start_vmm_prefix_size = Self::WARM_START_VMM_PAYLOAD_PREFIX_SIZE,
            tmp_dir_option = tmp_dir_option,
            help_option = help_option,
        );
    }

    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut benchmark_str: String = String::new();
        let mut hwloc_file: Option<String> = None;
        let mut iterations: usize = 100;
        let mut payload_size: Option<usize> = None;
        #[cfg(feature = "single-process")]
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
                Self::OPT_PAYLOAD_SIZE => {
                    i += 1;
                    if i >= args.len() {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "missing value for: {}",
                            Self::OPT_PAYLOAD_SIZE
                        ));
                    }
                    let parsed_payload_size: usize = args[i].parse::<usize>()?;
                    if parsed_payload_size == 0 {
                        Self::usage(args[0].as_str());
                        return Err(anyhow::anyhow!(
                            "{} must be a positive integer",
                            Self::OPT_PAYLOAD_SIZE,
                        ));
                    }
                    payload_size = Some(parsed_payload_size);
                },
                #[cfg(feature = "single-process")]
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
                // Reject -payload-size when it would be silently ignored.
                if payload_size.is_some()
                    && !matches!(
                        benchmark,
                        BenchmarkFlavour::WarmStart
                            | BenchmarkFlavour::WarmStartVMM
                            | BenchmarkFlavour::WarmStartSocket
                    )
                {
                    Self::usage(args[0].as_str());
                    return Err(anyhow::anyhow!(
                        "{benchmark} benchmark does not accept {}",
                        Self::OPT_PAYLOAD_SIZE,
                    ));
                }

                if let Some(payload_size) = payload_size
                    && matches!(benchmark, BenchmarkFlavour::WarmStartVMM)
                    && payload_size < Self::WARM_START_VMM_PAYLOAD_PREFIX_SIZE
                {
                    Self::usage(args[0].as_str());
                    return Err(anyhow::anyhow!(
                        "{benchmark} benchmark requires {} >= {} because the size includes the \
                         length prefix",
                        Self::OPT_PAYLOAD_SIZE,
                        Self::WARM_START_VMM_PAYLOAD_PREFIX_SIZE,
                    ));
                }

                Ok(Self {
                    benchmark,
                    hwloc_file,
                    iterations,
                    #[cfg(feature = "single-process")]
                    payload_size: payload_size.unwrap_or(DEFAULT_PAYLOAD_SIZE),
                    payload_size_override: payload_size,
                    #[cfg(feature = "single-process")]
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

    #[cfg(feature = "single-process")]
    pub fn payload_size(&self) -> usize {
        self.payload_size
    }

    pub fn payload_size_override(&self) -> Option<usize> {
        self.payload_size_override
    }

    #[cfg(feature = "single-process")]
    pub fn tmp_dir(&self) -> String {
        self.tmp_dir.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::Args;

    #[test]
    fn parse_rejects_zero_payload_size() {
        let args: Vec<String> = vec![
            "nanvix-bench".to_string(),
            "-benchmark".to_string(),
            "warm-start".to_string(),
            "-payload-size".to_string(),
            "0".to_string(),
        ];

        let error: anyhow::Error = match Args::parse(args) {
            Ok(_) => panic!("expected zero payload size to be rejected"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("positive integer"), "unexpected error: {error}");
    }

    #[test]
    fn parse_accepts_warm_start_vmm_payload_size_override() {
        let args: Vec<String> = vec![
            "nanvix-bench".to_string(),
            "-benchmark".to_string(),
            "warm-start-vmm".to_string(),
            "-payload-size".to_string(),
            "4096".to_string(),
        ];

        let args: Args = Args::parse(args).expect("expected warm-start-vmm payload size to parse");

        #[cfg(feature = "single-process")]
        assert_eq!(args.payload_size(), 4096);
        assert_eq!(args.payload_size_override(), Some(4096));
    }

    #[test]
    fn parse_accepts_warm_start_socket_payload_size_override() {
        let args: Vec<String> = vec![
            "nanvix-bench".to_string(),
            "-benchmark".to_string(),
            "warm-start-socket".to_string(),
            "-payload-size".to_string(),
            "4096".to_string(),
        ];

        let args: Args =
            Args::parse(args).expect("expected warm-start-socket payload size to parse");

        #[cfg(feature = "single-process")]
        assert_eq!(args.payload_size(), 4096);
        assert_eq!(args.payload_size_override(), Some(4096));
    }

    #[test]
    fn parse_keeps_warm_start_socket_payload_size_override_empty_when_omitted() {
        let args: Vec<String> = vec![
            "nanvix-bench".to_string(),
            "-benchmark".to_string(),
            "warm-start-socket".to_string(),
        ];

        let args: Args = Args::parse(args).expect("expected warm-start-socket to parse");

        assert_eq!(args.payload_size_override(), None);
    }

    #[test]
    fn parse_rejects_too_small_warm_start_vmm_payload_size() {
        let args: Vec<String> = vec![
            "nanvix-bench".to_string(),
            "-benchmark".to_string(),
            "warm-start-vmm".to_string(),
            "-payload-size".to_string(),
            "2".to_string(),
        ];

        let error: anyhow::Error = match Args::parse(args) {
            Ok(_) => panic!("expected too-small warm-start-vmm payload size to be rejected"),
            Err(error) => error,
        };

        assert!(error.to_string().contains(">= 4"), "unexpected error: {error}");
    }
}
