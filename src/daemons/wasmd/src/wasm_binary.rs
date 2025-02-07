// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # WASM Binary
//!
//! This module stores static information about the wasm binary that is embedded in the WASM Daemon
//! when the `wasm_binary` feature is enabled. This information is retrieved from the following
//! environment variables at build time:
//!
//! - `NANVIX_WASM_BINARY`: Full path to the WASM binary.
//! - `NANVIX_WASM_BINARY_BASENAME`: Name of the WASM binary.
//! - `NANVIX_WASM_BINARY_ARGS`: Arguments to pass to the WASM binary.
//!

/// Location of the WASM binary.
pub const WASM_BYTES: &[u8] = include_bytes!(env!("NANVIX_WASM_BINARY"));

/// Name of the WASM binary.
pub const WASM_BINARY_NAME: &str = env!("NANVIX_WASM_BINARY_BASENAME");

/// Arguments to pass to the WASM binary.
pub const WASM_BINARY_ARGS: &[&str] = &[env!("NANVIX_WASM_BINARY_ARGS")];
