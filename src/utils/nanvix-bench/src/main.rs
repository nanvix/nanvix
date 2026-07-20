// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod benchmark;
mod benchmarks;

//==================================================================================================
// Imports
//==================================================================================================

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
use ::std::{
    fs::File,
    io::BufReader,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "error";

/// Name of this package, used for logging and error messages.
const CARGO_PKG_NAME: &str = match option_env!("CARGO_PKG_NAME") {
    Some(cargo_pkg_name) => cargo_pkg_name,
    None => "nanvix-bench",
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[tokio::main]
async fn main() -> Result<()> {
    ::nanvix::log::init(false, DEFAULT_LOG_LEVEL, String::new(), None);

    match option_env!("RELEASE") {
        Some("yes") => {},
        Some(_) | None => {
            let reason: String =
                format!("{CARGO_PKG_NAME} requires Nanvix to be compiled with RELEASE=yes");
            error!("{reason}");
            anyhow::bail!(reason);
        },
    }

    match option_env!("LOG_LEVEL") {
        Some("panic") => {},
        Some(_) | None => {
            let reason: String =
                format!("{CARGO_PKG_NAME} requires Nanvix to be compiled with LOG_LEVEL=panic");
            error!("{reason}");
            anyhow::bail!(reason);
        },
    }

    let args: Args = Args::parse(::std::env::args().collect())?;
    if args.iterations() == 0 {
        let reason: String = format!("{CARGO_PKG_NAME} requires at least 1 iteration");
        error!("{reason}");
        anyhow::bail!(reason);
    }

    let hwloc: Option<HwLoc> = if let Some(hwloc_file_path) = args.hwloc_file() {
        let hwloc_file: File = File::open(hwloc_file_path)?;
        let hwloc_reader: BufReader<File> = BufReader::new(hwloc_file);
        Some(serde_json::from_reader(hwloc_reader)?)
    } else {
        None
    };
    if let Some(hwloc) = hwloc {
        hwloc::pin_main_thread(hwloc.get_client_core_str())?;
    }

    let mut benchmark: Benchmark = Benchmark {
        iterations: args.iterations(),
        payload_size_override: args.payload_size_override(),
        flavour: args.benchmark(),
        workspace_root: build_utils::find_workspace_root(),
    };

    let flavour: BenchmarkFlavour = benchmark.flavour.clone();
    let result: Result<()> = match flavour {
        BenchmarkFlavour::BootTime => benchmark.run_boot_time().await,
        BenchmarkFlavour::ColdStart => benchmark.run_cold_start_standalone().await,
        BenchmarkFlavour::ColdStartUvm => benchmark.run_cold_start_uvm().await,
        BenchmarkFlavour::SnapshotRestore => benchmark.run_snapshot_restore().await,
        BenchmarkFlavour::VfsBench => benchmark.run_vfs_bench_standalone().await,
        BenchmarkFlavour::WarmStartGateway => benchmark.run_warm_start_gateway().await,
        BenchmarkFlavour::WarmStartVMM => benchmark.run_warm_start_vmm().await,
        BenchmarkFlavour::WarmStartSocket => benchmark.run_warm_start_socket_standalone().await,
    };

    result.map_err(|error| {
        ::anyhow::anyhow!("error running benchmark {}: {error:?}", args.benchmark())
    })
}
