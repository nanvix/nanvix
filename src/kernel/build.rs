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
    collections::HashMap,
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
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Parses a string as either a hexadecimal (0x-prefixed) or decimal value.
///
/// # Parameters
///
/// - `value`: The string to parse.
/// - `key`: The key name, used in error messages.
///
/// # Returns
///
/// The parsed value.
///
fn parse_hex_or_decimal(value: &str, key: &str) -> usize {
    if let Some(stripped) = value.strip_prefix("0x") {
        usize::from_str_radix(stripped, 16)
            .unwrap_or_else(|_| panic!("Invalid hex value for {}: '{}'", key, value))
    } else {
        value
            .parse()
            .unwrap_or_else(|_| panic!("Invalid decimal value for {}: '{}'", key, value))
    }
}

///
/// # Description
///
/// Helper method to load a TOML file from a file path, and store it in a HashMap. This is a very
/// simple parser that only supports single-level TOMLs (i.e. no-nesting).
///
/// # Arguments
///
/// - `toml_path`: Path to the TOML file to load.
///
/// # Returns
///
/// A hash-map with the key-values in the TOML file.
///
fn load_toml(toml_path: &Path) -> HashMap<String, String> {
    let toml_content: String = fs::read_to_string(toml_path).expect("Failed to read TOML file");

    // Parse the config into a map
    let mut config: HashMap<String, String> = HashMap::new();
    for line in toml_content.lines() {
        let line: &str = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key: &str = key.trim();
            let value: &str = value.trim().trim_matches('"');
            config.insert(key.to_string(), value.to_string());
        }
    }
    config
}

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

    // Read kernel configuration
    let kernel_config_path: PathBuf = workspace_dir.join(DEFAULT_KERNEL_CONFIG_PATH);
    let kernel_config: HashMap<String, String> = load_toml(&kernel_config_path);

    /// Helper to retrieve a required key from the kernel config, panicking with a clear message
    /// if missing.
    fn required_key<'a>(config: &'a HashMap<String, String>, key: &str) -> &'a str {
        config
            .get(key)
            .unwrap_or_else(|| panic!("Missing required key '{}' in kernel_config.toml", key))
            .as_str()
    }

    // Extract kpool_base from config.
    let kpool_base: usize =
        parse_hex_or_decimal(required_key(&kernel_config, "kpool_base"), "kpool_base");

    // Tell Cargo to rerun build script if config changes.
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
    let linker_subdir: &str = if target_arch == "x86_64" {
        "x86_64"
    } else {
        "x86"
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
        .replace("@KPOOL_BASE@", &format!("{:#x}", kpool_base))
        .replace("@PLATFORM_BASE_ADDR@", &platform_base_addr)
        .replace("@TRAMPOLINE_ADDR@", &trampoline_addr);

    fs::write(&linker_output_path, linker_script).expect("Failed to write linker script");

    println!("cargo::rerun-if-changed={}", linker_template_path.display());
    println!("cargo::rustc-link-arg=-T{}", linker_output_path.display());
}
