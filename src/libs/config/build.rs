// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! This file implements the logic to load `.toml` files in the `build/` directory so that they can
//! be re-used as constants in rust source, and also in shell scripts.

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
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Helper method to load an TOML file from a file-path, and store it in a HashMap. This is a very
/// simple-parser that only supports single-level TOMLs (i.e. no-nesting). Concretely, this
/// function only supports parsing files with the following format:
///
/// ```toml
/// # Comment
/// key1 = val1
/// key2 = val2
///
/// # Another comment
/// ```
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
    let toml_content: String =
        fs::read_to_string(toml_path).expect("Failed to read kernel_config.toml");

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

fn generate_kernel_config(kernel_config_toml_path: &Path, kernel_config_output_path: &Path) {
    let kernel_config_toml: HashMap<String, String> = load_toml(kernel_config_toml_path);

    // Generate Rust constants from config.
    let mut constants = String::new();
    constants.push_str("pub mod kernel {\n");
    if let Some(memory_size) = kernel_config_toml.get("memory_size") {
        if let Ok(val) = memory_size.parse::<usize>() {
            constants.push_str(&format!("pub const MEMORY_SIZE: usize = {val};\n"));
        }
    }
    if let Some(kpool_size) = kernel_config_toml.get("kpool_size") {
        if let Ok(val) = kpool_size.parse::<usize>() {
            constants.push_str(&format!("pub const KPOOL_SIZE: usize = {val};\n"));
        }
    }
    if let Some(kstack_size) = kernel_config_toml.get("kstack_size") {
        if let Ok(val) = kstack_size.parse::<usize>() {
            constants.push_str(&format!("pub const KSTACK_SIZE: usize = {val};\n"));
        }
    }
    if let Some(timer_freq) = kernel_config_toml.get("timer_freq") {
        if let Ok(val) = timer_freq.parse::<u32>() {
            constants.push_str(&format!("pub const TIMER_FREQ: u32 = {val};\n"));
        }
    }
    if let Some(scheduler_freq) = kernel_config_toml.get("scheduler_freq") {
        if let Ok(val) = scheduler_freq.parse::<usize>() {
            constants.push_str(&format!("pub const SCHEDULER_FREQ: usize = {val};\n"));
        }
    }
    if let Some(max_ikc_messages) = kernel_config_toml.get("max_ikc_messages") {
        if let Ok(val) = max_ikc_messages.parse::<usize>() {
            constants.push_str(&format!("pub const MAX_IKC_MESSAGES: usize = {val};\n"));
        }
    }
    if let Some(ipc_message_size) = kernel_config_toml.get("ipc_message_size") {
        if let Ok(val) = ipc_message_size.parse::<usize>() {
            constants.push_str(&format!("pub const IPC_MESSAGE_SIZE: usize = {val};\n"));
        }
    }
    if let Some(max_mutexes) = kernel_config_toml.get("mutex_open_max") {
        if let Ok(val) = max_mutexes.parse::<usize>() {
            constants.push_str(&format!("pub const MUTEX_OPEN_MAX: usize = {val};\n"));
        }
    }
    if let Some(max_conditions) = kernel_config_toml.get("cond_open_max") {
        if let Ok(val) = max_conditions.parse::<usize>() {
            constants.push_str(&format!("pub const COND_OPEN_MAX: usize = {val};\n"));
        }
    }
    if let Some(ikc_poll_batch_size) = kernel_config_toml.get("ikc_poll_batch_size") {
        if let Ok(val) = ikc_poll_batch_size.parse::<usize>() {
            constants.push_str(&format!("pub const IKC_POLL_BATCH_SIZE: usize = {val};\n"));
        }
    }
    constants.push_str("}\n");

    // Write the generated file
    fs::write(kernel_config_output_path, constants).expect("Failed to write kernel_config.rs");
}

fn main() {
    // Read the TOML file using the workspace root for a reliable path
    let manifest_dir: String =
        env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
    let workspace_dir = Path::new(&manifest_dir)
        .ancestors()
        .nth(3)
        .expect("Failed to find workspace root");
    let out_dir: String = env::var("OUT_DIR").unwrap();

    // Parse kernel configuration file.
    let kernel_config_path: PathBuf = Path::new(&workspace_dir).join("build/kernel_config.toml");
    let kernel_dst_path: PathBuf = Path::new(&out_dir).join("kernel_config.rs");
    generate_kernel_config(&kernel_config_path, &kernel_dst_path);

    // Inform Cargo to rerun the build script if the TOML changes
    println!("cargo:rerun-if-changed=build/kernel_config.toml");
}
