// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Nanvix Daemon (nanvixd) entry point.
//!
//! This is the main executable for the Nanvix Daemon, which manages sandboxed execution
//! environments for user applications. It provides an HTTP API for creating and managing
//! user VM instances and handles their lifecycle.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::nanvix::{
    config::system::DEFAULT_MACHINE_NAME,
    http::HttpServer,
    log,
    log::error,
    registry::Registry,
    sandbox,
    sandbox::NAMED_RESOURCE_PREFIX,
    sandbox_cache::SandboxCacheConfig,
    terminal::Terminal,
};
use ::nanvixd::{
    args::Args,
    config::DEFAULT_TMP_DIRECTORY,
    tempdir::TemporaryDirectory,
};
use ::rand::{
    distr::Alphanumeric,
    Rng,
};
use ::std::{
    path::PathBuf,
    sync::{
        Arc,
        OnceLock,
    },
};
use ::tokio::fs;

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "info";

/// Binary name for Kernel.
const KERNEL_BINARY_NAME: &str = "kernel.elf";
/// Binary name for Linux Daemon.
#[cfg(not(feature = "single-process"))]
const LINUXD_BINARY_NAME: &str = "linuxd.elf";
/// Binary name for User VM.
#[cfg(not(feature = "single-process"))]
const USERVM_BINARY_NAME: &str = "uservm.elf";

/// Length of temporary directory random suffix.
const TMP_DIR_RANDOM_SUFFIX_LENGTH: usize = 4;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Global flag indicating whether the daemon is running in interactive mode. This flag is set
/// exactly once during initialization and remains immutable thereafter.
static INTERACTIVE_MODE: OnceLock<bool> = OnceLock::new();

//==================================================================================================
// Macros
//==================================================================================================

///
/// # Description
///
/// Logs a message using either `info!()` or `eprintln!()` depending on the mode.
///
/// # Parameters
///
/// - `fmt`: The format string.
/// - `args`: The format arguments.
///
macro_rules! log_info {
    ($fmt:expr $(, $($args:tt)*)?) => {
        if let Some(true) = $crate::INTERACTIVE_MODE.get().copied() {
            eprintln!($fmt $(, $($args)*)?);
        } else {
            ::nanvix::log::info!($fmt $(, $($args)*)?);
        }
    };
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for the Nanvix Daemon.
///
/// This function initializes the daemon by parsing command-line arguments, setting up logging,
/// configuring the sandbox cache, and starting the HTTP server to listen for client requests.
/// It runs until interrupted by a signal.
///
/// # Returns
///
/// On success, returns an empty tuple after graceful shutdown. On failure, returns an error
/// describing what went wrong during initialization or execution.
///
#[tokio::main]
pub async fn main() -> Result<()> {
    let args: Arc<Args> =
        Arc::new(Args::parse(std::env::args().filter(|s| !s.trim().is_empty()).collect())?);

    log::init(true, DEFAULT_LOG_LEVEL, args.log_directory().to_string());

    // Set the global INTERACTIVE_MODE flag.
    let _: Result<(), bool> = INTERACTIVE_MODE.set(args.interactive_mode());

    print_startup_info(&args);

    // Determine deployment type based on feature flag.
    #[cfg(feature = "single-process")]
    let deployment: &str = "single-process";
    #[cfg(not(feature = "single-process"))]
    let deployment: &str = "multi-process";

    // Determine target machine type from config.
    let machine: &str = DEFAULT_MACHINE_NAME;

    // Ensure all required binaries are available.
    #[cfg(feature = "single-process")]
    let (kernel_binary_path, _, _) =
        ensure_all_binaries_available(&args, machine, deployment).await?;

    #[cfg(not(feature = "single-process"))]
    let (kernel_binary_path, linuxd_binary_path, uservm_binary_path) =
        ensure_all_binaries_available(&args, machine, deployment).await?;

    // Create temporary directory that will be automatically cleaned up on drop.
    let tmp_directory: TemporaryDirectory = create_tmp_dir(DEFAULT_TMP_DIRECTORY).await?;

    let config: SandboxCacheConfig<()> = SandboxCacheConfig::new(
        args.control_plane_socket_type(),
        args.gateway_socket_type(),
        args.system_vm_socket_type(),
        args.console_file().clone(),
        args.hwloc().clone(),
        &kernel_binary_path,
        #[cfg(not(feature = "single-process"))]
        &linuxd_binary_path,
        #[cfg(not(feature = "single-process"))]
        &uservm_binary_path,
        #[cfg(feature = "single-process")]
        None,
        args.toolchain_binary_directory(),
        args.log_directory(),
        args.l2(),
        args.l2_snapshot_path(),
        tmp_directory.path().to_str().ok_or_else(|| {
            let reason: &str = "temporary directory path is not valid UTF-8";
            error!("main(): {reason}");
            anyhow::anyhow!(reason)
        })?,
    );

    // Remove dangling resources from previous runs. We do not expect concurrent instances of
    // nanvixd running in the same tmp directory, so we will not have unexpected side effects.
    sandbox::remove_dangling_resources(config.tmp_directory()).await?;

    // Check for interactive mode or HTTP mode.
    if let Some(true) = INTERACTIVE_MODE.get().copied() {
        let guest_binary_path: String = match args.program_name() {
            None => {
                let reason: &str = "no program name specified in interactive mode";
                error!("{reason}");
                anyhow::bail!(reason);
            },
            Some(path) => path.to_string(),
        };

        let guest_binary_args: String = if args.program_args().is_empty() {
            String::new()
        } else {
            args.program_args().join(" ")
        };

        let mut terminal: Terminal<()> = Terminal::new(config);
        if let Err(error) = terminal
            .run(None, None, &guest_binary_path, &guest_binary_args)
            .await
        {
            error!("terminal failed: {error}");
        }
    } else {
        let http_sockaddr: &str = match args.http_sockaddr() {
            None => {
                let reason: &str = "no HTTP socket address specified in HTTP mode";
                error!("{reason}");
                anyhow::bail!(reason);
            },
            Some(addr) => addr,
        };

        let mut http_server: HttpServer<()> = HttpServer::new(http_sockaddr, config);
        if let Err(error) = http_server.run().await {
            error!("http server failed: {error}");
        }
    }

    Ok(())
}

///
/// # Description
///
/// Ensures all required binaries are available. Checks if all binaries exist locally first.
/// If any binary is missing, fetches all of them from the nanvix-registry.
///
/// # Parameters
///
/// - `args`: The parsed command-line arguments.
/// - `machine`: The target machine type (e.g., `"microvm"`, `"hyperlight"`).
/// - `deployment`: The deployment type (e.g., `"single-process"`, `"multi-process"`).
///
/// # Returns
///
/// On success, returns a tuple containing paths to (kernel, linuxd, uservm) binaries.
/// On failure, returns an error describing what went wrong.
///
async fn ensure_all_binaries_available(
    args: &Args,
    machine: &str,
    deployment: &str,
) -> Result<(String, String, String)> {
    let kernel_binary_path: String = format!("{}/{}", args.binary_directory(), KERNEL_BINARY_NAME);

    #[cfg(not(feature = "single-process"))]
    let linuxd_binary_path: String = format!("{}/{}", args.binary_directory(), LINUXD_BINARY_NAME);

    #[cfg(not(feature = "single-process"))]
    let uservm_binary_path: String = format!("{}/{}", args.binary_directory(), USERVM_BINARY_NAME);

    // Check if all binaries are available locally.
    let kernel_available: bool = fs::metadata(&kernel_binary_path).await.is_ok();

    #[cfg(feature = "single-process")]
    let all_available: bool = kernel_available;

    #[cfg(not(feature = "single-process"))]
    let all_available: bool = {
        let linuxd_available: bool = fs::metadata(&linuxd_binary_path).await.is_ok();
        let uservm_available: bool = fs::metadata(&uservm_binary_path).await.is_ok();
        kernel_available && linuxd_available && uservm_available
    };

    // If all binaries are available locally, use them.
    if all_available {
        log_info!("using local binary {}: {}", KERNEL_BINARY_NAME, kernel_binary_path);

        #[cfg(not(feature = "single-process"))]
        {
            log_info!("using local binary {}: {}", LINUXD_BINARY_NAME, linuxd_binary_path);
            log_info!("using local binary {}: {}", USERVM_BINARY_NAME, uservm_binary_path);
        }

        #[cfg(feature = "single-process")]
        return Ok((kernel_binary_path, String::new(), String::new()));

        #[cfg(not(feature = "single-process"))]
        return Ok((kernel_binary_path, linuxd_binary_path, uservm_binary_path));
    }

    log_info!("not all binaries found locally, fetching all from registry");

    let registry: Registry = Registry::new();

    let kernel_cached_path: String = registry
        .get_cached_binary(machine, deployment, KERNEL_BINARY_NAME)
        .await?;
    log_info!("using registry binary {}: {}", KERNEL_BINARY_NAME, kernel_cached_path);

    #[cfg(feature = "single-process")]
    return Ok((kernel_cached_path, String::new(), String::new()));

    #[cfg(not(feature = "single-process"))]
    {
        let linuxd_cached_path: String = registry
            .get_cached_binary(machine, deployment, LINUXD_BINARY_NAME)
            .await?;
        log_info!("using registry binary {}: {}", LINUXD_BINARY_NAME, linuxd_cached_path);

        let uservm_cached_path: String = registry
            .get_cached_binary(machine, deployment, USERVM_BINARY_NAME)
            .await?;
        log_info!("using registry binary {}: {}", USERVM_BINARY_NAME, uservm_cached_path);

        Ok((kernel_cached_path, linuxd_cached_path, uservm_cached_path))
    }
}

///
/// # Description
///
/// Prints startup information for the Nanvix Daemon.
///
/// This function displays the version, deployment type, operation mode, and L2 status.
///
/// # Parameters
///
/// - `args`: The parsed command-line arguments.
///
fn print_startup_info(args: &Args) {
    let mode: &str = if args.interactive_mode() {
        "interactive"
    } else {
        "http"
    };

    #[cfg(feature = "single-process")]
    log_info!("nanvixd {}, single-process deployment, {} mode", env!("CARGO_PKG_VERSION"), mode);

    #[cfg(not(feature = "single-process"))]
    log_info!(
        "nanvixd {}, multi-process deployment, {} mode, l2 {}",
        env!("CARGO_PKG_VERSION"),
        mode,
        if args.l2() { "enabled" } else { "disabled" }
    );
}

///
/// # Description
///
/// Creates a temporary directory for the sandbox cache.
///
/// This function generates a random 4-character alphanumeric directory name under the specified
/// tmp directory path. The directory will be automatically cleaned up when the returned
/// `TemporaryDirectory` is dropped.
///
/// # Parameters
///
/// - `tmp_directory`: The base temporary directory path.
///
/// # Returns
///
/// On success, returns a `TemporaryDirectory` instance that manages the lifecycle of the created
/// directory. On failure, returns an error describing what went wrong during directory creation.
///
async fn create_tmp_dir(tmp_directory: &str) -> Result<TemporaryDirectory> {
    let tmp_dirname: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(TMP_DIR_RANDOM_SUFFIX_LENGTH)
        .map(char::from)
        .collect();
    let tmp_directory_path: PathBuf =
        PathBuf::from(tmp_directory).join(format!("{NAMED_RESOURCE_PREFIX}:{}", tmp_dirname));

    // Check if temporary directory already exists (very unlikely).
    if tmp_directory_path.exists() {
        let reason: String =
            format!("unique temporary directory already exists (path={tmp_directory_path:?})");
        error!("create_tmp_dir(): {reason}");
        anyhow::bail!(reason);
    }

    let tmp_directory: TemporaryDirectory = TemporaryDirectory::new(tmp_directory_path).await?;

    Ok(tmp_directory)
}
