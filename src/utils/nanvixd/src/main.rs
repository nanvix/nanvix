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
use ::config::system::DEFAULT_MACHINE_NAME;
use ::nanvix_registry::Registry;
use ::nanvix_sandbox_cache::SandboxCacheConfig;
use ::nanvix_terminal::Terminal;
use ::nanvixd::{
    args::Args,
    http::HttpServer,
};
use ::std::sync::Arc;
use ::syslog::{
    error,
    info,
};
use ::tokio::fs;

//==================================================================================================
// Constants
//==================================================================================================

/// Binary name for Kernel.
const KERNEL_BINARY_NAME: &str = "kernel.elf";
/// Binary name for Linux Daemon.
#[cfg(not(feature = "single-process"))]
const LINUXD_BINARY_NAME: &str = "linuxd.elf";
/// Binary name for User VM.
#[cfg(not(feature = "single-process"))]
const USERVM_BINARY_NAME: &str = "uservm.elf";

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

    ::syslog::init(true, args.log_directory().to_string());

    #[cfg(feature = "single-process")]
    info!("nanvixd {} single-process mode", env!("CARGO_PKG_VERSION"));
    #[cfg(not(feature = "single-process"))]
    info!("nanvixd {} multi-process mode", env!("CARGO_PKG_VERSION"));

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

    let config: SandboxCacheConfig = SandboxCacheConfig::new(
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
        args.tmp_directory(),
    );

    // Check for interactive mode or HTTP mode.
    if args.interactive_mode() {
        info!("running in interactive mode");
        let guest_binary_path: String = match args.program_name() {
            None => {
                let reason: &str = "no program name specified in interactive mode";
                error!("{}", reason);
                anyhow::bail!(reason);
            },
            Some(path) => path.to_string(),
        };

        let guest_binary_args: String = if args.program_args().is_empty() {
            String::new()
        } else {
            args.program_args().join(" ")
        };

        let mut terminal: Terminal = Terminal::new(config);
        if let Err(error) = terminal.run(&guest_binary_path, &guest_binary_args).await {
            error!("terminal failed: {}", error);
        }
    } else if args.http_mode() {
        info!("running in HTTP mode");
        let http_sockaddr: &str = match args.http_sockaddr() {
            None => {
                let reason: &str = "no HTTP socket address specified in HTTP mode";
                error!("{}", reason);
                anyhow::bail!(reason);
            },
            Some(addr) => addr,
        };

        let mut http_server: HttpServer = HttpServer::new(http_sockaddr, config);
        if let Err(error) = http_server.run().await {
            error!("http server failed: {}", error);
        }
    } else {
        let reason: &str = "no operation mode specified";
        error!("{}", reason);
        anyhow::bail!(reason);
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
        info!("All required binaries found locally");
        info!("Using local binary {}: {}", KERNEL_BINARY_NAME, kernel_binary_path);

        #[cfg(not(feature = "single-process"))]
        {
            info!("Using local binary {}: {}", LINUXD_BINARY_NAME, linuxd_binary_path);
            info!("Using local binary {}: {}", USERVM_BINARY_NAME, uservm_binary_path);
        }

        #[cfg(feature = "single-process")]
        return Ok((kernel_binary_path, String::new(), String::new()));

        #[cfg(not(feature = "single-process"))]
        return Ok((kernel_binary_path, linuxd_binary_path, uservm_binary_path));
    }

    info!("Not all binaries found locally, fetching all from registry");

    let registry: Registry = Registry::new();

    let kernel_cached_path: String = registry
        .get_cached_binary(machine, deployment, KERNEL_BINARY_NAME)
        .await?;
    info!("Using registry binary {}: {}", KERNEL_BINARY_NAME, kernel_cached_path);

    #[cfg(feature = "single-process")]
    return Ok((kernel_cached_path, String::new(), String::new()));

    #[cfg(not(feature = "single-process"))]
    {
        let linuxd_cached_path: String = registry
            .get_cached_binary(machine, deployment, LINUXD_BINARY_NAME)
            .await?;
        info!("Using registry binary {}: {}", LINUXD_BINARY_NAME, linuxd_cached_path);

        let uservm_cached_path: String = registry
            .get_cached_binary(machine, deployment, USERVM_BINARY_NAME)
            .await?;
        info!("Using registry binary {}: {}", USERVM_BINARY_NAME, uservm_cached_path);

        Ok((kernel_cached_path, linuxd_cached_path, uservm_cached_path))
    }
}
