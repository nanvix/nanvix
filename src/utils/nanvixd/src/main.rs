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
#![forbid(clippy::unwrap_used)]
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

#[cfg(all(feature = "standalone", feature = "multi-process"))]
compile_error!("features `standalone` and `multi-process` are mutually exclusive");

#[cfg(all(feature = "standalone", feature = "single-process"))]
compile_error!("features `standalone` and `single-process` are mutually exclusive");

#[cfg(all(feature = "single-process", feature = "multi-process"))]
compile_error!("features `single-process` and `multi-process` are mutually exclusive");

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    info,
};
#[cfg(feature = "multi-process")]
use ::nanvix::sandbox_config::SandboxCacheConfig;
#[cfg(feature = "single-process")]
use ::nanvix::sandbox_config::SimpleSandboxCacheConfig;
#[cfg(feature = "standalone")]
use ::nanvix::sandbox_config::StandaloneConfig;
use ::nanvix::{
    config::system::DEFAULT_MACHINE_NAME,
    http::HttpServer,
    sandbox::NAMED_RESOURCE_PREFIX,
    terminal::Terminal,
};
use ::nanvixd::{
    args::Args,
    tempdir::TemporaryDirectory,
};
use ::std::{
    path::PathBuf,
    process::ExitCode,
    sync::Arc,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};
use ::tokio::fs;

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "info";

/// Maximum exit code value that can be represented as a process exit code.
/// Exit codes are clamped to the range [0, 255] for compatibility with POSIX systems.
const MAX_EXIT_CODE: i32 = 255;

/// Binary name for Kernel.
const KERNEL_BINARY_NAME: &str = "kernel.elf";
/// Binary name for Linux Daemon.
#[cfg(feature = "multi-process")]
const LINUXD_BINARY_NAME: &str = "linuxd.elf";
/// Binary name for User VM.
#[cfg(feature = "multi-process")]
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
/// On success, returns the exit code of the workload in interactive mode or `ExitCode::SUCCESS`
/// in HTTP mode. On failure, returns an error describing what went wrong.
///
// # NOTES
//
// - We build the tokio runtime manually instead of using `#[tokio::main]` because the macro
//   expansion emits `#[allow(clippy::expect_used)]`, which is incompatible with our crate-level
//   `#![forbid(clippy::expect_used)]` lint configuration.
///
pub fn main() -> Result<ExitCode> {
    let rt: ::tokio::runtime::Runtime = ::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| ::anyhow::anyhow!("failed to build tokio runtime: {e}"))?;
    rt.block_on(async_main())
}

/// # Description
///
/// Asynchronous entry point for the nanvixd daemon.
///
async fn async_main() -> Result<ExitCode> {
    let args: Arc<Args> =
        Arc::new(Args::parse(std::env::args().filter(|s| !s.trim().is_empty()).collect())?);

    ::nanvix::log::init(
        !args.log_to_stdout(),
        DEFAULT_LOG_LEVEL,
        args.log_directory().to_string(),
        None,
    );

    print_startup_info(&args);

    // Ensure all required binaries are available.
    #[cfg(any(feature = "single-process", feature = "standalone"))]
    let (kernel_binary_path, _, _) = ensure_all_binaries_available(&args).await?;

    #[cfg(feature = "multi-process")]
    let (kernel_binary_path, linuxd_binary_path, uservm_binary_path) =
        ensure_all_binaries_available(&args).await?;

    // Create temporary directory that will be automatically cleaned up on drop.
    // Standalone mode does not use the temporary directory, so skip creating it
    // to avoid unnecessary filesystem overhead on the cold-start path.
    #[cfg(not(feature = "standalone"))]
    let tmp_directory: TemporaryDirectory = create_tmp_dir(args.tmp_directory()).await?;

    #[cfg(feature = "single-process")]
    let config: SimpleSandboxCacheConfig<()> = SimpleSandboxCacheConfig::new(
        args.control_plane_socket_type(),
        args.gateway_socket_type(),
        args.system_vm_socket_type(),
        args.console_file().clone(),
        args.ramfs_filename().map(|s| s.to_string()),
        args.hwloc().clone(),
        &kernel_binary_path,
        None,
        args.log_directory(),
        tmp_directory.path().to_str().ok_or_else(|| {
            let reason: &str = "temporary directory path is not valid UTF-8";
            error!("main(): {reason}");
            anyhow::anyhow!(reason)
        })?,
        args.networking_mode(),
    );

    #[cfg(feature = "standalone")]
    let config: StandaloneConfig = StandaloneConfig::new(
        kernel_binary_path,
        args.ramfs_filename().map(|s| s.to_string()),
        args.console_file().clone(),
        args.snapshot_path().map(|s| s.to_string()),
        args.mount_directory().map(|s| s.to_string()),
        args.kernel_args().map(|s| s.to_string()),
        args.networking_mode(),
        args.host_filter(),
        #[cfg(feature = "gdb")]
        args.gdb_port(),
        args.gateway_sockaddr().map(|s| s.to_string()),
    );

    #[cfg(feature = "multi-process")]
    let config: SandboxCacheConfig<()> = SandboxCacheConfig::new(
        args.control_plane_socket_type(),
        args.gateway_socket_type(),
        args.system_vm_socket_type(),
        args.console_file().clone(),
        args.ramfs_filename().map(|s| s.to_string()),
        args.hwloc().clone(),
        &kernel_binary_path,
        &linuxd_binary_path,
        &uservm_binary_path,
        args.log_directory(),
        tmp_directory.path().to_str().ok_or_else(|| {
            let reason: &str = "temporary directory path is not valid UTF-8";
            error!("main(): {reason}");
            anyhow::anyhow!(reason)
        })?,
        args.networking_mode(),
    );

    // Check for interactive mode or HTTP mode.
    if args.interactive_mode() {
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

        // In single-process mode, the terminal connects through the simplified sandbox cache
        // (which embeds linuxd as an async task).
        #[cfg(feature = "single-process")]
        let mut terminal: Terminal<()> = Terminal::new(config);
        // In standalone mode, the terminal drives the VM directly (no linuxd).
        #[cfg(feature = "standalone")]
        let mut terminal: Terminal = Terminal::new(config);
        // In multi-process mode, the terminal connects through the sandbox cache.
        #[cfg(feature = "multi-process")]
        let mut terminal: Terminal<()> = Terminal::new(config);
        let exit_code: i32 = terminal
            .run(None, None, &guest_binary_path, &guest_binary_args)
            .await?;

        // Clamp exit code to valid range [0, 255] for POSIX compatibility.
        // Negative values become 255 (error), values > 255 are clamped to 255.
        let clamped_exit_code: u8 = if !(0..=MAX_EXIT_CODE).contains(&exit_code) {
            MAX_EXIT_CODE as u8
        } else {
            exit_code as u8
        };

        Ok(ExitCode::from(clamped_exit_code))
    } else {
        // HTTP mode.

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
            return Ok(ExitCode::FAILURE);
        }

        Ok(ExitCode::SUCCESS)
    }
}

///
/// # Description
///
/// Ensures all required binaries are available. Checks if all binaries exist locally first.
/// If any binary is missing, fails with an error listing the missing binaries.
///
/// # Parameters
///
/// - `args`: The parsed command-line arguments.
///
/// # Returns
///
/// On success, returns a tuple containing paths to (kernel, linuxd, uservm) binaries.
/// On failure, returns an error describing what went wrong.
///
async fn ensure_all_binaries_available(args: &Args) -> Result<(String, String, String)> {
    let kernel_binary_path: String = format!("{}/{}", args.binary_directory(), KERNEL_BINARY_NAME);

    #[cfg(feature = "multi-process")]
    let linuxd_binary_path: String = format!("{}/{}", args.binary_directory(), LINUXD_BINARY_NAME);

    #[cfg(feature = "multi-process")]
    let uservm_binary_path: String = format!("{}/{}", args.binary_directory(), USERVM_BINARY_NAME);

    // Check if all binaries are available locally.
    let kernel_metadata_result: Result<std::fs::Metadata, std::io::Error> =
        fs::metadata(&kernel_binary_path).await;
    let kernel_available: bool = kernel_metadata_result.is_ok();

    #[cfg(any(feature = "single-process", feature = "standalone"))]
    let all_available: bool = kernel_available;

    #[cfg(feature = "multi-process")]
    let all_available: bool = {
        let linuxd_available: bool = fs::metadata(&linuxd_binary_path).await.is_ok();
        let uservm_available: bool = fs::metadata(&uservm_binary_path).await.is_ok();
        kernel_available && linuxd_available && uservm_available
    };

    // If all binaries are available locally, use them.
    if all_available {
        info!("using local binary {}: {}", KERNEL_BINARY_NAME, kernel_binary_path);

        #[cfg(feature = "multi-process")]
        {
            info!("using local binary {}: {}", LINUXD_BINARY_NAME, linuxd_binary_path);
            info!("using local binary {}: {}", USERVM_BINARY_NAME, uservm_binary_path);
        }

        #[cfg(any(feature = "single-process", feature = "standalone"))]
        return Ok((kernel_binary_path, String::new(), String::new()));

        #[cfg(feature = "multi-process")]
        return Ok((kernel_binary_path, linuxd_binary_path, uservm_binary_path));
    }

    // Standalone mode requires all binaries to be available locally.
    #[cfg(feature = "standalone")]
    {
        // Safety: we only reach here when kernel_available is false, so the result must be Err.
        let reason: String = match kernel_metadata_result {
            Err(err) => format!(
                "kernel binary not available locally (required in standalone mode): {}: {}",
                kernel_binary_path, err
            ),
            Ok(_) => unreachable!(),
        };
        error!("ensure_all_binaries_available(): {reason}");
        anyhow::bail!(reason);
    }

    #[cfg(not(feature = "standalone"))]
    {
        let mut missing: Vec<&str> = Vec::new();
        if !kernel_available {
            missing.push(KERNEL_BINARY_NAME);
        }

        #[cfg(feature = "multi-process")]
        {
            if fs::metadata(&linuxd_binary_path).await.is_err() {
                missing.push(LINUXD_BINARY_NAME);
            }
            if fs::metadata(&uservm_binary_path).await.is_err() {
                missing.push(USERVM_BINARY_NAME);
            }
        }

        let reason: String =
            format!("required binaries not available locally: {}", missing.join(", "));
        error!("ensure_all_binaries_available(): {reason}");
        anyhow::bail!(reason);
    }
}

///
/// # Description
///
/// Prints startup information for the Nanvix Daemon.
///
/// This function displays the version, deployment type, operation mode, and machine type.
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
    info!(
        "nanvixd {}, single-process deployment, {} mode, machine {}",
        env!("CARGO_PKG_VERSION"),
        mode,
        DEFAULT_MACHINE_NAME
    );

    #[cfg(feature = "standalone")]
    info!(
        "nanvixd {}, standalone deployment, {} mode, machine {}",
        env!("CARGO_PKG_VERSION"),
        mode,
        DEFAULT_MACHINE_NAME
    );

    #[cfg(feature = "standalone")]
    if let Some(snapshot) = args.snapshot_path() {
        info!("snapshot restore from: {}", snapshot);
    }

    #[cfg(feature = "multi-process")]
    info!(
        "nanvixd {}, multi-process deployment, {} mode, machine {}",
        env!("CARGO_PKG_VERSION"),
        mode,
        DEFAULT_MACHINE_NAME
    );
}

///
/// # Description
///
/// Creates a temporary directory for the sandbox cache.
///
/// This function generates a unique directory name using a timestamp encoded in base64url
/// format (RFC 4648 Section 5) with filename-safe characters: A-Z, a-z, 0-9, -, _.
/// The timestamp is based on microseconds since UNIX_EPOCH. The directory will be
/// automatically cleaned up when the returned `TemporaryDirectory` is dropped.
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
#[cfg_attr(feature = "standalone", allow(dead_code))]
async fn create_tmp_dir(tmp_directory: &str) -> Result<TemporaryDirectory> {
    // Get current timestamp in microseconds.
    let timestamp_micros: u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            let reason: String = format!("failed to get system time: {e}");
            error!("create_tmp_dir(): {reason}");
            anyhow::anyhow!(reason)
        })?
        .as_micros();

    // Encode timestamp in base64url using RFC 4648 compliant filename-safe characters.
    let tmp_dirname: String = encode_base64_filename(timestamp_micros);
    #[cfg(unix)]
    let tmp_directory_path: PathBuf =
        PathBuf::from(tmp_directory).join(format!("{NAMED_RESOURCE_PREFIX}:{}", tmp_dirname));
    #[cfg(windows)]
    let tmp_directory_path: PathBuf =
        PathBuf::from(tmp_directory).join(format!("{NAMED_RESOURCE_PREFIX}-{}", tmp_dirname));

    // Check if temporary directory already exists (extremely unlikely with timestamp-based naming).
    if tmp_directory_path.exists() {
        let reason: String =
            format!("unique temporary directory already exists (path={tmp_directory_path:?})");
        error!("create_tmp_dir(): {reason}");
        anyhow::bail!(reason);
    }

    let tmp_directory: TemporaryDirectory = TemporaryDirectory::new(tmp_directory_path).await?;

    Ok(tmp_directory)
}

///
/// # Description
///
/// Encodes a number in base64url format as defined in RFC 4648 Section 5.
///
/// This function converts a 128-bit unsigned integer to a base64url string representation
/// using the URL and filename-safe alphabet (RFC 4648 Table 2): A-Z (indices 0-25),
/// a-z (indices 26-51), 0-9 (indices 52-61), - (index 62), and _ (index 63).
/// These characters are safe to use in URLs and filenames across all common operating systems.
///
/// # Parameters
///
/// - `mut num`: The number to encode.
///
/// # Returns
///
/// A base64url-encoded string representation of the input number using RFC 4648 compliant
/// filename-safe characters. Zero is encoded as `"A"`.
///
fn encode_base64_filename(mut num: u128) -> String {
    const BASE64_CHARS: &[char] = &[
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
        'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1',
        '2', '3', '4', '5', '6', '7', '8', '9', '-', '_',
    ];
    const BASE: u128 = 64;

    // Handle the zero case.
    if num == 0 {
        return "A".to_string();
    }

    let mut result: Vec<char> = Vec::new();
    while num > 0 {
        result.push(BASE64_CHARS[(num % BASE) as usize]);
        num /= BASE;
    }

    result.reverse();
    result.iter().collect()
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::needless_range_loop)]
mod tests {
    use super::*;

    ///
    /// # Description
    ///
    /// Tests that encoding zero returns "A" as specified in the function documentation.
    ///
    #[test]
    fn test_encode_base64_filename_zero() {
        assert_eq!(encode_base64_filename(0), "A");
    }

    ///
    /// # Description
    ///
    /// Tests encoding of various numbers to verify correct base64url conversion.
    ///
    #[test]
    fn test_encode_base64_filename_various_numbers() {
        // Test first 64 values map to single characters in the alphabet.
        assert_eq!(encode_base64_filename(1), "B");
        assert_eq!(encode_base64_filename(25), "Z");
        assert_eq!(encode_base64_filename(26), "a");
        assert_eq!(encode_base64_filename(51), "z");
        assert_eq!(encode_base64_filename(52), "0");
        assert_eq!(encode_base64_filename(61), "9");
        assert_eq!(encode_base64_filename(62), "-");
        assert_eq!(encode_base64_filename(63), "_");

        // Test multi-character encodings.
        assert_eq!(encode_base64_filename(64), "BA");
        assert_eq!(encode_base64_filename(65), "BB");
        assert_eq!(encode_base64_filename(127), "B_");
        assert_eq!(encode_base64_filename(128), "CA");
        assert_eq!(encode_base64_filename(4095), "__");
        assert_eq!(encode_base64_filename(4096), "BAA");

        // Test larger values.
        assert_eq!(encode_base64_filename(1000000), "D0JA");
        assert_eq!(encode_base64_filename(u64::MAX as u128), "P__________");
    }

    ///
    /// # Description
    ///
    /// Tests that character ordering in output follows RFC 4648 base64url alphabet ordering.
    /// Verifies that sequential numbers produce lexicographically ordered strings when appropriate.
    ///
    #[test]
    fn test_encode_base64_filename_character_ordering() {
        // Verify RFC 4648 alphabet ordering: A-Z (0-25), a-z (26-51), 0-9 (52-61), - (62), _ (63).
        let encodings: Vec<String> = (0..64).map(encode_base64_filename).collect();

        // First 26 should be A-Z.
        for i in 0..26 {
            assert_eq!(encodings[i].len(), 1);
            assert_eq!(encodings[i].as_bytes()[0], b'A' + i as u8);
        }

        // Next 26 should be a-z.
        for i in 26..52 {
            assert_eq!(encodings[i].len(), 1);
            assert_eq!(encodings[i].as_bytes()[0], b'a' + (i - 26) as u8);
        }

        // Next 10 should be 0-9.
        for i in 52..62 {
            assert_eq!(encodings[i].len(), 1);
            assert_eq!(encodings[i].as_bytes()[0], b'0' + (i - 52) as u8);
        }

        // Last two should be - and _.
        assert_eq!(encodings[62], "-");
        assert_eq!(encodings[63], "_");
    }

    ///
    /// # Description
    ///
    /// Tests uniqueness guarantees by verifying that different inputs produce different outputs.
    /// This is critical for using the function to generate unique directory names.
    ///
    #[test]
    fn test_encode_base64_filename_uniqueness() {
        use ::std::collections::HashSet;

        // Test uniqueness for a range of consecutive numbers.
        let mut seen: HashSet<String> = HashSet::new();
        for i in 0..10000 {
            let encoded: String = encode_base64_filename(i);
            assert!(seen.insert(encoded.clone()), "duplicate encoding for {}: {}", i, encoded);
        }

        // Test uniqueness for sparse large numbers.
        let test_values: Vec<u128> = vec![
            0,
            1,
            u64::MAX as u128,
            u64::MAX as u128 + 1,
            u128::MAX / 2,
            u128::MAX - 1,
            u128::MAX,
        ];

        let mut seen_large: HashSet<String> = HashSet::new();
        for value in test_values {
            let encoded: String = encode_base64_filename(value);
            assert!(
                seen_large.insert(encoded.clone()),
                "duplicate encoding for {}: {}",
                value,
                encoded
            );
        }
    }

    ///
    /// # Description
    ///
    /// Tests that encoded strings only contain RFC 4648 base64url filename-safe characters.
    ///
    #[test]
    fn test_encode_base64_filename_valid_characters() {
        use ::std::collections::HashSet;

        let valid_chars: HashSet<char> =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
                .chars()
                .collect();

        let test_values: Vec<u128> = vec![0, 1, 63, 64, 4095, 4096, 1000000, u128::MAX];

        for value in test_values {
            let encoded: String = encode_base64_filename(value);
            for ch in encoded.chars() {
                assert!(
                    valid_chars.contains(&ch),
                    "invalid character '{}' in encoding of {}",
                    ch,
                    value
                );
            }
        }
    }

    ///
    /// # Description
    ///
    /// Tests boundary conditions and edge cases.
    ///
    #[test]
    fn test_encode_base64_filename_boundary_conditions() {
        // Minimum value.
        assert_eq!(encode_base64_filename(0), "A");

        // Maximum value.
        let max_encoding: String = encode_base64_filename(u128::MAX);
        assert!(!max_encoding.is_empty());
        assert!(max_encoding.len() <= 22); // log64(2^128) ≈ 21.33.

        // Power of 64 values.
        assert_eq!(encode_base64_filename(64), "BA");
        assert_eq!(encode_base64_filename(64 * 64), "BAA");
        assert_eq!(encode_base64_filename(64 * 64 * 64), "BAAA");
    }

    ///
    /// # Description
    ///
    /// Tests that `ensure_all_binaries_available` succeeds when the kernel binary exists locally.
    ///
    #[cfg(feature = "standalone")]
    #[tokio::test]
    async fn test_ensure_all_binaries_available_kernel_present() {
        use ::tempfile::TempDir;

        let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
        let kernel_path: PathBuf = tmp_dir.path().join(KERNEL_BINARY_NAME);
        std::fs::write(&kernel_path, b"fake kernel").expect("failed to write kernel binary");

        let args: Args = Args::parse(vec![
            "nanvixd".to_string(),
            "-bin-dir".to_string(),
            tmp_dir.path().to_str().expect("invalid path").to_string(),
            "--".to_string(),
            "hello".to_string(),
        ])
        .expect("failed to parse args");

        let result = ensure_all_binaries_available(&args).await;
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);

        let (kernel, linuxd, uservm) = result.expect("already checked");
        assert_eq!(kernel, kernel_path.to_str().expect("invalid path"));
        assert!(linuxd.is_empty());
        assert!(uservm.is_empty());
    }

    ///
    /// # Description
    ///
    /// Tests that `ensure_all_binaries_available` fails with a descriptive error when the kernel
    /// binary does not exist locally in standalone mode.
    ///
    #[cfg(feature = "standalone")]
    #[tokio::test]
    async fn test_ensure_all_binaries_available_kernel_missing() {
        use ::tempfile::TempDir;

        let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");

        let args: Args = Args::parse(vec![
            "nanvixd".to_string(),
            "-bin-dir".to_string(),
            tmp_dir.path().to_str().expect("invalid path").to_string(),
            "--".to_string(),
            "hello".to_string(),
        ])
        .expect("failed to parse args");

        let result = ensure_all_binaries_available(&args).await;
        assert!(result.is_err(), "expected Err, got: {:?}", result);

        let err_msg: String = format!("{}", result.expect_err("already checked"));
        assert!(
            err_msg.contains("not available locally"),
            "error should mention binary not available locally, got: {err_msg}"
        );
        assert!(
            err_msg.contains(KERNEL_BINARY_NAME),
            "error should mention kernel binary path, got: {err_msg}"
        );
    }
}
