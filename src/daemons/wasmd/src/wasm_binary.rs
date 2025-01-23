// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Location of the WASM binary.
pub const WASM_BYTES: &[u8] = include_bytes!("../build/bin/hello-wasm.wasm");

/// Name of the WASM binary.
pub const WASM_BINARY_NAME: &str = "hello-wasm.wasm";

/// Arguments to pass to the WASM binary.
pub const WASM_BINARY_ARGS: &[&str] = &[];
