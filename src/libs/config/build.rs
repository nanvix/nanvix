// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    env,
    fs,
    path::Path,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn main() {
    // Read the TOML file
    let kernel_config_content =
        fs::read_to_string("build/kernel_config.toml").expect("Failed to read kernel_config.toml");
    let kernel_config: toml::Value = kernel_config_content.parse().expect("Invalid TOML format");

    // Prepare the output path
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("kernel_config.rs");

    // Generate Rust constants from TOML
    let mut constants = String::new();
    constants.push_str("pub mod kernel {\n");
    if let Some(memory_size) = kernel_config
        .get("memory_size")
        .and_then(|v| v.as_integer())
    {
        constants.push_str(&format!("pub const MEMORY_SIZE: usize = {};\n", memory_size));
    }
    if let Some(kpool_size) = kernel_config.get("kpool_size").and_then(|v| v.as_integer()) {
        constants.push_str(&format!("pub const KPOOL_SIZE: usize = {};\n", kpool_size));
    }
    if let Some(kstack_size) = kernel_config
        .get("kstack_size")
        .and_then(|v| v.as_integer())
    {
        constants.push_str(&format!("pub const KSTACK_SIZE: usize = {};\n", kstack_size));
    }
    if let Some(timer_freq) = kernel_config.get("timer_freq").and_then(|v| v.as_integer()) {
        constants.push_str(&format!("pub const TIMER_FREQ: u32 = {};\n", timer_freq));
    }
    if let Some(scheduler_freq) = kernel_config
        .get("scheduler_freq")
        .and_then(|v| v.as_integer())
    {
        constants.push_str(&format!("pub const SCHEDULER_FREQ: usize = {};\n", scheduler_freq));
    }
    if let Some(max_ikc_messages) = kernel_config
        .get("max_ikc_messages")
        .and_then(|v| v.as_integer())
    {
        constants.push_str(&format!("pub const MAX_IKC_MESSAGES: usize = {};\n", max_ikc_messages));
    }
    if let Some(ipc_message_size) = kernel_config
        .get("ipc_message_size")
        .and_then(|v| v.as_integer())
    {
        constants.push_str(&format!("pub const IPC_MESSAGE_SIZE: usize = {};\n", ipc_message_size));
    }
    if let Some(max_mutexes) = kernel_config
        .get("mutex_open_max")
        .and_then(|v| v.as_integer())
    {
        constants.push_str(&format!("pub const MUTEX_OPEN_MAX: usize = {};\n", max_mutexes));
    }
    if let Some(max_conditions) = kernel_config
        .get("cond_open_max")
        .and_then(|v| v.as_integer())
    {
        constants.push_str(&format!("pub const COND_OPEN_MAX: usize = {};\n", max_conditions));
    }
    constants.push_str("}\n");

    // Write the generated file
    fs::write(&dest_path, constants).expect("Failed to write kernel_config.rs");

    // Inform Cargo to rerun the build script if the TOML changes
    println!("cargo:rerun-if-changed=build/kernel_config.toml");
}
