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
    sandbox::NAMED_RESOURCE_PREFIX,
    sandbox_cache::SandboxCacheConfig,
    terminal::Terminal,
};
use ::nanvixd::{
    args::Args,
    config::DEFAULT_TMP_DIRECTORY,
    tempdir::TemporaryDirectory,
};
use ::std::{
    path::PathBuf,
    sync::{
        Arc,
        OnceLock,
    },
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

/// Binary name for Kernel.
const KERNEL_BINARY_NAME: &str = "kernel.elf";
/// Binary name for Linux Daemon.
#[cfg(not(feature = "single-process"))]
const LINUXD_BINARY_NAME: &str = "linuxd.elf";
/// Binary name for User VM.
#[cfg(not(feature = "single-process"))]
const USERVM_BINARY_NAME: &str = "uservm.elf";

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

    log::init(true, DEFAULT_LOG_LEVEL, args.log_directory().to_string(), None);

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
        args.ramfs_filename().map(|s| s.to_string()),
        args.hwloc().clone(),
        args.netns_pool_size(),
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

    let registry: Registry = Registry::new(None);

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
/// This function displays the version, deployment type, operation mode, L2 status, and machine type.
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
    log_info!(
        "nanvixd {}, single-process deployment, {} mode, machine {}",
        env!("CARGO_PKG_VERSION"),
        mode,
        DEFAULT_MACHINE_NAME
    );

    #[cfg(not(feature = "single-process"))]
    log_info!(
        "nanvixd {}, multi-process deployment, {} mode, l2 {}, machine {}",
        env!("CARGO_PKG_VERSION"),
        mode,
        if args.l2() { "enabled" } else { "disabled" },
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
    let tmp_directory_path: PathBuf =
        PathBuf::from(tmp_directory).join(format!("{NAMED_RESOURCE_PREFIX}:{}", tmp_dirname));

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
            assert_eq!(encodings[i].chars().next().unwrap() as u8, b'A' + i as u8);
        }

        // Next 26 should be a-z.
        for i in 26..52 {
            assert_eq!(encodings[i].len(), 1);
            assert_eq!(encodings[i].chars().next().unwrap() as u8, b'a' + (i - 26) as u8);
        }

        // Next 10 should be 0-9.
        for i in 52..62 {
            assert_eq!(encodings[i].len(), 1);
            assert_eq!(encodings[i].chars().next().unwrap() as u8, b'0' + (i - 52) as u8);
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
}
