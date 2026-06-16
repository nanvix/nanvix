// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
// These two allows are required because we conditionally compile the benchmarks based on whether
// the right compilation flags are used.
#![allow(dead_code)]
#![allow(unreachable_code)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod benchmark;
mod benchmarks;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(any(feature = "multi-process", feature = "single-process"))]
use crate::benchmark::{
    LinuxdDeployment,
    UserVmDeployment,
};
use crate::{
    args::Args,
    benchmark::{
        Benchmark,
        BenchmarkFlavour,
    },
};
use ::anyhow::Result;
use ::log::error;
use ::nanvix::{
    hwloc,
    hwloc::HwLoc,
};
#[cfg(any(feature = "multi-process", feature = "single-process"))]
use ::std::time::Duration;
use ::std::{
    fs::File,
    io::BufReader,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "error";

// Name of this package, used for logging and error messages.
const CARGO_PKG_NAME: &str = match option_env!("CARGO_PKG_NAME") {
    Some(cargo_pkg_name) => cargo_pkg_name,
    None => "nanvix-bench",
};

///
/// # Description
///
/// Timeout (in seconds) for HTTP requests to nanvixd (start, kill, etc.).
///
#[cfg(any(feature = "multi-process", feature = "single-process"))]
const NANVIXD_HTTP_TIMEOUT_SECS: u64 = 60;

//==================================================================================================
// Validation
//==================================================================================================

/// Validates that the selected benchmark is compatible with the compile-time feature set.
fn validate_benchmark(flavour: &BenchmarkFlavour) -> Result<()> {
    let has_multi = cfg!(feature = "multi-process");
    let has_single = cfg!(feature = "single-process");
    let has_standalone = cfg!(feature = "standalone");
    let has_l2 = cfg!(feature = "l2");
    let has_timestamp = cfg!(feature = "timestamp-messages");

    // System-level benchmarks (those using nanvixd) need multi-process, single-process, or
    // standalone.
    if flavour.needs_nanvixd() {
        if !has_multi && !has_single && !has_standalone {
            anyhow::bail!(
                "benchmark '{flavour}' requires compilation with multi-process, single-process, \
                 or standalone"
            );
        }

        // L2 benchmarks additionally need the l2 feature.
        if flavour.is_l2() && !has_l2 {
            anyhow::bail!("benchmark '{flavour}' requires compilation with l2");
        }

        // concurrent (non-L2) needs multi-process specifically.
        if matches!(flavour, BenchmarkFlavour::Concurrent) && !has_multi {
            anyhow::bail!("benchmark '{flavour}' requires compilation with multi-process");
        }

        // In standalone mode, only ColdStart and VfsBench are supported (no HTTP-based
        // benchmarks). Both spawn nanvixd in interactive mode rather than using the HTTP API.
        if has_standalone
            && !has_multi
            && !has_single
            && !matches!(flavour, BenchmarkFlavour::ColdStart | BenchmarkFlavour::VfsBench)
        {
            anyhow::bail!(
                "benchmark '{flavour}' is not supported in standalone mode (only cold-start and \
                 vfs-bench are available)"
            );
        }
    }

    // echo-breakdown benchmarks require timestamp-messages; all others reject it.
    if flavour.requires_timestamp_messages() {
        if !has_timestamp {
            anyhow::bail!(
                "benchmark '{flavour}' requires Nanvix (re-) compilation with TIMESTAMP_MSG=yes"
            );
        }
    } else if has_timestamp {
        anyhow::bail!("benchmark '{flavour}' must be compiled with TIMESTAMP_MSG=no (or omit it)");
    }

    Ok(())
}

//==================================================================================================
// Main
//==================================================================================================

#[tokio::main]
async fn main() -> Result<()> {
    ::nanvix::log::init(false, DEFAULT_LOG_LEVEL, String::new(), None);

    // Check if RELEASE=yes was set at build time.
    match option_env!("RELEASE") {
        Some("yes") => {},
        Some(_) | None => {
            let reason: String =
                format!("{CARGO_PKG_NAME} requires Nanvix to be compiled with RELEASE=yes");
            error!("{reason}");
            anyhow::bail!(reason);
        },
    }

    // Check if LOG_LEVEL was set at build time and ensure it is "panic".
    match option_env!("LOG_LEVEL") {
        Some("panic") => {},
        Some(_) | None => {
            let reason: String =
                format!("{CARGO_PKG_NAME} requires Nanvix to be compiled with LOG_LEVEL=panic");
            error!("{reason}");
            anyhow::bail!(reason);
        },
    }

    let args: Args = Args::parse(std::env::args().collect())?;

    // Validate that iterations is at least 1 to avoid panicking on percentile indexing.
    if args.iterations() == 0 {
        let reason: String = format!("{CARGO_PKG_NAME} requires at least 1 iteration");
        error!("{reason}");
        anyhow::bail!(reason);
    }

    // Validate that the selected benchmark is compatible with the compile-time feature set.
    validate_benchmark(&args.benchmark())?;

    // Parse hwloc from JSON file.
    let hwloc: Option<HwLoc> = if let Some(hwloc_file_path) = args.hwloc_file() {
        let hwloc_file: File = File::open(hwloc_file_path)?;
        let hwloc_reader: BufReader<File> = BufReader::new(hwloc_file);
        Some(serde_json::from_reader(hwloc_reader)?)
    } else {
        None
    };

    // Initialize HwLoc and pin main thread.
    if let Some(hwloc) = hwloc.clone() {
        hwloc::pin_main_thread(hwloc.get_client_core_str())?;
    }

    let mut benchmark = Benchmark {
        iterations: args.iterations(),
        hwloc_file: args.hwloc_file(),
        hwloc,
        flavour: args.benchmark(),
        workspace_root: build_utils::find_workspace_root(),
        #[cfg(any(feature = "multi-process", feature = "single-process"))]
        nanvixd: None,
        #[cfg(any(feature = "multi-process", feature = "single-process"))]
        nanvixd_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(NANVIXD_HTTP_TIMEOUT_SECS))
            .build()?,
        #[cfg(any(feature = "multi-process", feature = "single-process"))]
        nanvixd_clh_bin_path: args.clh_bin_path(),
        #[cfg(any(feature = "multi-process", feature = "single-process"))]
        nanvixd_netns_pool_size: args.netns_pool_size(),
        #[cfg(any(feature = "multi-process", feature = "single-process"))]
        nanvixd_tmp_dir: args.tmp_dir(),
        #[cfg(any(feature = "multi-process", feature = "single-process"))]
        user_vm_id: None,
    };

    #[cfg(any(feature = "multi-process", feature = "single-process"))]
    let deployment: LinuxdDeployment = benchmark.flavour.deployment();
    let result: Result<(), anyhow::Error> = match &benchmark.flavour {
        BenchmarkFlavour::BootTime => benchmark.run_boot_time().await,
        BenchmarkFlavour::ColdStart => {
            #[cfg(feature = "standalone")]
            {
                benchmark.run_cold_start_standalone().await
            }
            #[cfg(not(feature = "standalone"))]
            {
                benchmark
                    .run_cold_start(&deployment, &UserVmDeployment::OneToOne)
                    .await
            }
        },
        BenchmarkFlavour::ColdStartL2 => {
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            {
                benchmark
                    .run_cold_start(&deployment, &UserVmDeployment::OneToOne)
                    .await
            }
            #[cfg(not(any(feature = "multi-process", feature = "single-process")))]
            {
                anyhow::bail!("cold-start-l2 requires multi-process or single-process")
            }
        },
        BenchmarkFlavour::ColdStartUvm => {
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            {
                benchmark
                    .run_cold_start(&deployment, &UserVmDeployment::PreWarm)
                    .await
            }
            #[cfg(not(any(feature = "multi-process", feature = "single-process")))]
            {
                anyhow::bail!("cold-start-uvm requires multi-process or single-process")
            }
        },
        BenchmarkFlavour::EchoBreakdown | BenchmarkFlavour::EchoBreakdownL2 => {
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            {
                benchmark.run_echo_breakdown(&deployment).await
            }
            #[cfg(not(any(feature = "multi-process", feature = "single-process")))]
            {
                anyhow::bail!("echo-breakdown requires multi-process or single-process")
            }
        },
        BenchmarkFlavour::RoundTripLatency => {
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            {
                benchmark.run_round_trip_latency(&deployment).await
            }
            #[cfg(not(any(feature = "multi-process", feature = "single-process")))]
            {
                anyhow::bail!("round-trip-latency requires multi-process or single-process")
            }
        },
        BenchmarkFlavour::Concurrent | BenchmarkFlavour::ConcurrentL2 => {
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            {
                if let Some(num_concurrent_vms) = args.num_concurrent_vms() {
                    benchmark
                        .run_concurrent(&deployment, num_concurrent_vms)
                        .await
                } else {
                    anyhow::bail!("this benchmark must be run with a set number of concurrent VMs");
                }
            }
            #[cfg(not(any(feature = "multi-process", feature = "single-process")))]
            {
                anyhow::bail!("concurrent requires multi-process or single-process")
            }
        },
        BenchmarkFlavour::WarmStart | BenchmarkFlavour::WarmStartL2 => {
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            {
                benchmark.run_warm_start(&deployment).await
            }
            #[cfg(not(any(feature = "multi-process", feature = "single-process")))]
            {
                anyhow::bail!("warm-start requires multi-process or single-process")
            }
        },
        BenchmarkFlavour::WarmStartVMM => benchmark.run_warm_start_vmm().await,
        BenchmarkFlavour::VfsBench => {
            #[cfg(feature = "standalone")]
            {
                benchmark.run_vfs_bench_standalone().await
            }
            #[cfg(not(feature = "standalone"))]
            {
                anyhow::bail!("vfs-bench requires compilation with standalone")
            }
        },
        BenchmarkFlavour::SnapshotRestore => benchmark.run_snapshot_restore().await,
    };
    match result {
        Ok(()) => {},
        Err(e) => {
            // In case of an error, re-run the clean up to prevent having dangling processes. Note
            // that the clean up is idempotent.
            #[cfg(any(feature = "multi-process", feature = "single-process"))]
            benchmark.cleanup();

            anyhow::bail!("error running benchmark {}: {e:?}", args.benchmark());
        },
    }

    Ok(())
}
