// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use std::{
    env,
    fs::File,
    io::Read,
    path::{
        Path,
        PathBuf,
    },
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    //==============================================================================================
    //Benchmark-Specific Environment Variables
    //==============================================================================================

    // Get CARGO_MANIFEST_DIR Environment Variable.
    let manifest_dir: String = match env::var("CARGO_MANIFEST_DIR") {
        Ok(mdir) => mdir,
        Err(_) => panic!("failed to get CARGO_MANIFEST_DIR environment variable"),
    };

    let config_path: PathBuf = Path::new(&manifest_dir).join("config.json");
    if config_path.exists() {
        println!("cargo:rerun-if-changed=./config.json");
        let mut file: File = File::open(&config_path).expect("Failed to open file");
        let mut raw_content: String = String::new();
        let _ = file.read_to_string(&mut raw_content);
        let content = raw_content.replace("\n", "").replace(" ", "");
        println!("cargo:rustc-env=CONFIG={content}");
    }

    //==============================================================================================
    // Build-Time Seed for Random Walks
    //==============================================================================================

    // The seed introduces non-determinism between builds: callers can pin a specific seed by
    // exporting `NANVIX_BENCH_SEED`; otherwise a fresh seed is derived from the current wall-clock
    // time when the build script runs. The seed is baked into the binary at compile time, so a
    // given binary always replays the same random walk.
    println!("cargo:rerun-if-env-changed=NANVIX_BENCH_SEED");
    let seed: String = env::var("NANVIX_BENCH_SEED").unwrap_or_else(|_| {
        // Combine whole seconds and sub-second nanoseconds without a u128 -> u64 truncation cast.
        // `as_secs()` is already u64, and `subsec_nanos()` is u32 (< 1e9, fits in u64). Wrapping
        // arithmetic is intentional: any wall-clock time produces a non-trivial seed.
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| {
                d.as_secs()
                    .wrapping_mul(1_000_000_000)
                    .wrapping_add(u64::from(d.subsec_nanos()))
            })
            .unwrap_or(0xDEADBEEFCAFEBABE_u64)
            .to_string()
    });
    println!("cargo:rustc-env=NANVIX_BENCH_SEED={seed}");

    //==============================================================================================
    // Link Archive
    //==============================================================================================

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");

    //==============================================================================================
}
