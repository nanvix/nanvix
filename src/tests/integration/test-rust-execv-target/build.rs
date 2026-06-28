// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use std::env;

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    // Link against the user-space linker script for the target architecture.
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");

    // Strip all symbols and debug information at link time. This program is loaded at runtime by
    // the execv() test, which stages its whole image into a bounded kernel region; stripping keeps
    // the image small (debug builds otherwise carry megabytes of debug info) without depending on
    // an external objcopy/llvm-tools being installed.
    println!("cargo::rustc-link-arg=-s");
}
