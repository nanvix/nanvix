// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![deny(clippy::all)]

use std::env;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");

    // Strip all symbols and debug information at link time. fork-exec-loop-test execv()s this image
    // ITERATIONS times, and execv() stages the whole on-disk image into the guest page-by-page over
    // IPC; an unstripped debug build carries megabytes of debug info, so loading it repeatedly is
    // slow enough to exceed the integration-test timeout. Stripping keeps the image small (matching
    // execv-target) without depending on an external objcopy/llvm-tools being installed.
    println!("cargo::rustc-link-arg=-s");
}
