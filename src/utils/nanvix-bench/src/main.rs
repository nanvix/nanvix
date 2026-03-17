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

#[cfg(not(feature = "timestamp-messages"))]
use crate::benchmark::UserVmDeployment;
use crate::{
    args::Args,
    benchmark::{
        Benchmark,
        BenchmarkFlavour,
        LinuxdDeployment,
    },
};
use ::anyhow::Result;
use ::log::error;
use ::nanvix::{
    hwloc,
    hwloc::HwLoc,
};
use ::std::{
    fs::File,
    io::BufReader,
    time::Duration,
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
const NANVIXD_HTTP_TIMEOUT_SECS: u64 = 60;

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
        nanvixd: None,
        nanvixd_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(NANVIXD_HTTP_TIMEOUT_SECS))
            .build()?,
        nanvixd_toolchain_bin_dir: args.toolchain_bin_dir(),
        nanvixd_netns_pool_size: args.netns_pool_size(),
        nanvixd_tmp_dir: args.tmp_dir(),
        user_vm_id: None,
    };

    let result = match benchmark.flavour {
        BenchmarkFlavour::BootTime => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_boot_time().await
            }
        },
        BenchmarkFlavour::ColdStart => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark
                    .run_cold_start(&LinuxdDeployment::Process, &UserVmDeployment::OneToOne)
                    .await
            }
        },
        #[cfg(feature = "l2")]
        BenchmarkFlavour::ColdStartL2 => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "l2"))]
            {
                anyhow::bail!(
                    "WARNING: this benchmark requires Nanvix (re-) compilation with \
                     DEPLOYMENT_MODE=l2"
                );
            }

            #[cfg(all(not(feature = "timestamp-messages"), feature = "l2"))]
            {
                benchmark
                    .run_cold_start(&LinuxdDeployment::L2Vm, &UserVmDeployment::OneToOne)
                    .await
            }
        },
        BenchmarkFlavour::ColdStartUvm => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark
                    .run_cold_start(&LinuxdDeployment::Process, &UserVmDeployment::PreWarm)
                    .await
            }
        },
        BenchmarkFlavour::EchoBreakdown => {
            #[cfg(not(feature = "timestamp-messages"))]
            {
                anyhow::bail!(
                    "WARNING: this benchmark requires Nanvix (re-) compilation with \
                     TIMESTAMP_MSG=yes"
                );
            }

            #[cfg(feature = "timestamp-messages")]
            {
                benchmark
                    .run_echo_breakdown(&LinuxdDeployment::Process)
                    .await
            }
        },
        #[cfg(feature = "l2")]
        BenchmarkFlavour::EchoBreakdownL2 => {
            #[cfg(not(feature = "timestamp-messages"))]
            {
                anyhow::bail!(
                    "WARNING: this benchmark requires Nanvix (re-) compilation with \
                     TIMESTAMP_MSG=yes"
                );
            }

            #[cfg(not(feature = "l2"))]
            {
                anyhow::bail!(
                    "WARNING: this benchmark requires Nanvix (re-) compilation with \
                     DEPLOYMENT_MODE=l2"
                );
            }

            #[cfg(all(feature = "timestamp-messages", feature = "l2"))]
            {
                benchmark.run_echo_breakdown(&LinuxdDeployment::L2Vm).await
            }
        },
        BenchmarkFlavour::RoundTripLatency => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark
                    .run_round_trip_latency(&LinuxdDeployment::Process)
                    .await
            }
        },
        BenchmarkFlavour::Concurrent => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                if let Some(num_concurrent_vms) = args.num_concurrent_vms() {
                    benchmark
                        .run_concurrent(&LinuxdDeployment::Process, num_concurrent_vms)
                        .await
                } else {
                    anyhow::bail!("this benchmark must be run with a set number of concurrent VMs");
                }
            }
        },
        #[cfg(feature = "l2")]
        BenchmarkFlavour::ConcurrentL2 => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "l2"))]
            {
                anyhow::bail!(
                    "WARNING: this benchmark requires Nanvix (re-) compilation with \
                     DEPLOYMENT_MODE=l2"
                );
            }

            #[cfg(all(not(feature = "timestamp-messages"), feature = "l2"))]
            {
                if let Some(num_concurrent_vms) = args.num_concurrent_vms() {
                    benchmark
                        .run_concurrent(&LinuxdDeployment::L2Vm, num_concurrent_vms)
                        .await
                } else {
                    anyhow::bail!("this benchmark must be run with a set number of concurrent VMs");
                }
            }
        },
        BenchmarkFlavour::WarmStart => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_warm_start(&LinuxdDeployment::Process).await
            }
        },
        #[cfg(feature = "l2")]
        BenchmarkFlavour::WarmStartL2 => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "l2"))]
            {
                anyhow::bail!(
                    "WARNING: this benchmark requires Nanvix (re-) compilation with \
                     DEPLOYMENT_MODE=l2"
                );
            }

            #[cfg(all(not(feature = "timestamp-messages"), feature = "l2"))]
            {
                benchmark.run_warm_start(&LinuxdDeployment::L2Vm).await
            }
        },
        BenchmarkFlavour::WarmStartVMM => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_warm_start_vmm().await
            }
        },
        BenchmarkFlavour::SnapshotRestore => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_snapshot_restore().await
            }
        },
    };
    match result {
        Ok(()) => {},
        Err(e) => {
            // In case of an error, re-run the clean up to prevent having dangling processes. Note
            // that the clean up is idempotent.
            benchmark.cleanup();

            anyhow::bail!("error running benchmark {}: {e:?}", args.benchmark());
        },
    }

    Ok(())
}
