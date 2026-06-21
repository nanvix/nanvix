// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![deny(clippy::all)]

use std::env;

fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");
}
