// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    env,
    fs,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default path for `kernel_config.toml` file.
const DEFAULT_KERNEL_CONFIG_PATH: &str = "build/kernel_config.toml";

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    //==============================================================================================
    // Get Essential Environment Variables
    //==============================================================================================

    // Get OUT_DIR environment variable.
    let out_dir: String = match env::var("OUT_DIR") {
        Ok(out_dir) => out_dir,
        Err(_) => panic!("failed to get OUT_DIR environment variable"),
    };

    //==============================================================================================
    // Read Kernel Configuration
    //==============================================================================================

    // Find the workspace root by locating the Cargo.toml with [workspace].
    let workspace_dir: PathBuf = build_utils::find_workspace_root();

    // Tell Cargo to rerun build script if config changes.
    let kernel_config_path: PathBuf = workspace_dir.join(DEFAULT_KERNEL_CONFIG_PATH);
    println!("cargo::rerun-if-changed={}", kernel_config_path.display());

    //==============================================================================================
    // Generate Linker Script
    //==============================================================================================

    // Generate linker script from template with machine-specific values.
    //
    // MACHINE_RESERVED is the space reserved after __KERNEL_END for machine-specific structures.
    // No reserved space is needed for microvm.
    let target_arch: String =
        env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    let linker_subdir: &str = match target_arch.as_str() {
        "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        _ => "x86",
    };
    let linker_template_path: PathBuf =
        workspace_dir.join(format!("build/kernel/linker/{}/kernel.ld.in", linker_subdir));
    let linker_output_path: PathBuf = Path::new(&out_dir).join("kernel.ld");

    let machine_reserved: String = "0x0".to_string();
    let platform_base_addr: String = "0x0".to_string();

    // Trampoline is offset to 0x8000 for AP real-mode startup.
    let trampoline_addr: String = format!("{:#x}", 0x8000usize);

    let linker_template: String =
        fs::read_to_string(&linker_template_path).expect("Failed to read linker script template");
    let linker_script: String = linker_template
        .replace("@MACHINE_RESERVED@", &machine_reserved)
        .replace("@PLATFORM_BASE_ADDR@", &platform_base_addr)
        .replace("@TRAMPOLINE_ADDR@", &trampoline_addr);

    fs::write(&linker_output_path, linker_script).expect("Failed to write linker script");

    println!("cargo::rerun-if-changed={}", linker_template_path.display());
    println!("cargo::rustc-link-arg=-T{}", linker_output_path.display());
}
