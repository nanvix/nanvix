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
    terminal::Terminal,
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
        if let Err(error) = terminal.run(guest_binary_path, guest_binary_args).await {
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
