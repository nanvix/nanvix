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
// Modules
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::nanvix_sandbox_cache::SandboxCacheConfig;
use ::nanvixd::{
    args::Args,
    http::HttpServer,
};
use ::std::sync::Arc;
use ::syslog::{
    error,
    info,
};

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

    ::syslog::init(args.log_to_file(), args.log_directory().to_string());

    #[cfg(feature = "single-process")]
    info!("nanvixd {} single-process mode", env!("CARGO_PKG_VERSION"));
    #[cfg(not(feature = "single-process"))]
    info!("nanvixd {} multi-process mode", env!("CARGO_PKG_VERSION"));

    let config: SandboxCacheConfig = SandboxCacheConfig::new(
        args.control_plane_socket_type(),
        args.gateway_socket_type(),
        args.system_vm_socket_type(),
        args.console_file().clone(),
        args.hwloc().clone(),
        args.binary_directory(),
        args.toolchain_binary_directory(),
        args.log_directory(),
        args.l2(),
        args.tmp_directory(),
    );

    let mut http_server: HttpServer = HttpServer::new(args.http_sockaddr(), config);
    if let Err(error) = http_server.run().await {
        error!("HTTP server failed: {}", error);
    }

    Ok(())
}
