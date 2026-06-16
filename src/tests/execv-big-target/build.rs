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

    // Strip symbols and debug information at link time so the on-disk image is dominated by the
    // intentional large data blob rather than incidental debug info, and so it does not depend on
    // an external objcopy/llvm-tools being installed.
    println!("cargo::rustc-link-arg=-s");
}
