// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Nanvix Daemon (nanvixd) entry point.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::unwrap_used)]
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    info,
};
use ::nanvix::{
    config::system::DEFAULT_MACHINE_NAME,
    http::HttpServer,
    sandbox_config::StandaloneConfig,
    terminal::Terminal,
};
use ::nanvixd::args::Args;
use ::std::process::ExitCode;
use ::tokio::fs;

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "info";

/// Maximum exit code value that can be represented as a process exit code.
const MAX_EXIT_CODE: i32 = 255;

/// Binary name for Kernel.
const KERNEL_BINARY_NAME: &str = "kernel.elf";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Entry point for the Nanvix Daemon.
///
/// # Returns
///
/// On success, returns the workload exit code in interactive mode or success in HTTP mode.
///
pub fn main() -> Result<ExitCode> {
    let runtime: ::tokio::runtime::Runtime = ::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| ::anyhow::anyhow!("failed to build tokio runtime: {error}"))?;
    runtime.block_on(async_main())
}

///
/// # Description
///
/// Asynchronous entry point for the Nanvix Daemon.
///
async fn async_main() -> Result<ExitCode> {
    let args: Args = Args::parse(
        ::std::env::args()
            .filter(|arg| !arg.trim().is_empty())
            .collect(),
    )?;

    ::nanvix::log::init(
        !args.log_to_stdout(),
        DEFAULT_LOG_LEVEL,
        args.log_directory().to_string(),
        None,
    );

    print_startup_info(&args);

    let kernel_binary_path: String = ensure_kernel_binary_available(&args).await?;
    let config: StandaloneConfig = StandaloneConfig::new(
        kernel_binary_path,
        args.ramfs_filename().map(str::to_string),
        args.console_file(),
        args.snapshot_path().map(str::to_string),
        args.mount_directory().map(str::to_string),
        args.kernel_args().map(str::to_string),
        args.networking_mode(),
        args.host_filter(),
        #[cfg(feature = "gdb")]
        args.gdb_port(),
        args.gateway_sockaddr().map(str::to_string),
    );

    if args.interactive_mode() {
        let guest_binary_path: &str = match args.program_name() {
            Some(path) => path,
            None => {
                let reason: &str = "no program name specified in interactive mode";
                error!("{reason}");
                anyhow::bail!(reason);
            },
        };
        let guest_binary_args: String = args.program_args().join(" ");
        let mut terminal: Terminal = Terminal::new(config);
        let exit_code: i32 = terminal
            .run(None, None, guest_binary_path, &guest_binary_args)
            .await?;
        let exit_code: u8 = if (0..=MAX_EXIT_CODE).contains(&exit_code) {
            exit_code as u8
        } else {
            MAX_EXIT_CODE as u8
        };
        Ok(ExitCode::from(exit_code))
    } else {
        let http_sockaddr: &str = match args.http_sockaddr() {
            Some(sockaddr) => sockaddr,
            None => {
                let reason: &str = "no HTTP socket address specified in HTTP mode";
                error!("{reason}");
                anyhow::bail!(reason);
            },
        };
        let mut http_server: HttpServer = HttpServer::new(http_sockaddr, config);
        if let Err(error) = http_server.run().await {
            error!("http server failed: {error}");
            return Ok(ExitCode::FAILURE);
        }
        Ok(ExitCode::SUCCESS)
    }
}

///
/// # Description
///
/// Ensures that the kernel binary is available locally.
///
/// # Parameters
///
/// - `args`: Parsed command-line arguments.
///
/// # Returns
///
/// On success, returns the kernel binary path. On failure, returns an error.
///
async fn ensure_kernel_binary_available(args: &Args) -> Result<String> {
    let kernel_binary_path: String = format!("{}/{}", args.binary_directory(), KERNEL_BINARY_NAME);
    match fs::metadata(&kernel_binary_path).await {
        Ok(_) => {
            info!("using local binary {KERNEL_BINARY_NAME}: {kernel_binary_path}");
            Ok(kernel_binary_path)
        },
        Err(error) => {
            let reason: String =
                format!("kernel binary not available locally: {kernel_binary_path}: {error}");
            error!("ensure_kernel_binary_available(): {reason}");
            Err(::anyhow::anyhow!(reason))
        },
    }
}

///
/// # Description
///
/// Prints startup information for the Nanvix Daemon.
///
/// # Parameters
///
/// - `args`: Parsed command-line arguments.
///
fn print_startup_info(args: &Args) {
    let operation_mode: &str = if args.interactive_mode() {
        "interactive"
    } else {
        "http"
    };
    info!(
        "nanvixd {}, standalone deployment, {} mode, machine {}",
        env!("CARGO_PKG_VERSION"),
        operation_mode,
        DEFAULT_MACHINE_NAME
    );

    if let Some(snapshot) = args.snapshot_path() {
        info!("snapshot restore from: {snapshot}");
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::std::path::PathBuf;
    use ::tempfile::TempDir;

    #[tokio::test]
    async fn ensure_kernel_binary_available_accepts_existing_kernel() {
        let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
        let kernel_path: PathBuf = tmp_dir.path().join(KERNEL_BINARY_NAME);
        ::std::fs::write(&kernel_path, b"fake kernel").expect("failed to write kernel binary");
        let args: Args = Args::parse(vec![
            "nanvixd".to_string(),
            "-bin-dir".to_string(),
            tmp_dir.path().to_string_lossy().into_owned(),
            "--".to_string(),
            "hello".to_string(),
        ])
        .expect("failed to parse args");

        let result: String = ensure_kernel_binary_available(&args)
            .await
            .expect("kernel should be available");
        assert_eq!(result, kernel_path.to_string_lossy());
    }

    #[tokio::test]
    async fn ensure_kernel_binary_available_rejects_missing_kernel() {
        let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
        let args: Args = Args::parse(vec![
            "nanvixd".to_string(),
            "-bin-dir".to_string(),
            tmp_dir.path().to_string_lossy().into_owned(),
            "--".to_string(),
            "hello".to_string(),
        ])
        .expect("failed to parse args");

        let error: ::anyhow::Error = ensure_kernel_binary_available(&args)
            .await
            .expect_err("missing kernel should fail");
        let message: String = error.to_string();
        assert!(message.contains("not available locally"));
        assert!(message.contains(KERNEL_BINARY_NAME));
    }
}
