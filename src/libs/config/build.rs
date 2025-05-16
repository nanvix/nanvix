// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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

fn main() {
    // Read the TOML file
    let kernel_config_content =
        fs::read_to_string("build/kernel_config.toml").expect("Failed to read kernel_config.toml");

    // Prepare the output path
    let out_dir: String = env::var("OUT_DIR").unwrap();
    let dest_path: PathBuf = Path::new(&out_dir).join("kernel_config.rs");

    // Parse the config into a map
    let mut config: HashMap<String, String> = std::collections::HashMap::new();
    for line in kernel_config_content.lines() {
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

    // Generate Rust constants from config.
    let mut constants = String::new();
    constants.push_str("pub mod kernel {\n");
    if let Some(memory_size) = config.get("memory_size") {
        if let Ok(val) = memory_size.parse::<usize>() {
            constants.push_str(&format!("pub const MEMORY_SIZE: usize = {};\n", val));
        }
    }
    if let Some(kpool_size) = config.get("kpool_size") {
        if let Ok(val) = kpool_size.parse::<usize>() {
            constants.push_str(&format!("pub const KPOOL_SIZE: usize = {};\n", val));
        }
    }
    if let Some(kstack_size) = config.get("kstack_size") {
        if let Ok(val) = kstack_size.parse::<usize>() {
            constants.push_str(&format!("pub const KSTACK_SIZE: usize = {};\n", val));
        }
    }
    if let Some(timer_freq) = config.get("timer_freq") {
        if let Ok(val) = timer_freq.parse::<u32>() {
            constants.push_str(&format!("pub const TIMER_FREQ: u32 = {};\n", val));
        }
    }
    if let Some(scheduler_freq) = config.get("scheduler_freq") {
        if let Ok(val) = scheduler_freq.parse::<usize>() {
            constants.push_str(&format!("pub const SCHEDULER_FREQ: usize = {};\n", val));
        }
    }
    if let Some(max_ikc_messages) = config.get("max_ikc_messages") {
        if let Ok(val) = max_ikc_messages.parse::<usize>() {
            constants.push_str(&format!("pub const MAX_IKC_MESSAGES: usize = {};\n", val));
        }
    }
    if let Some(ipc_message_size) = config.get("ipc_message_size") {
        if let Ok(val) = ipc_message_size.parse::<usize>() {
            constants.push_str(&format!("pub const IPC_MESSAGE_SIZE: usize = {};\n", val));
        }
    }
    if let Some(max_mutexes) = config.get("mutex_open_max") {
        if let Ok(val) = max_mutexes.parse::<usize>() {
            constants.push_str(&format!("pub const MUTEX_OPEN_MAX: usize = {};\n", val));
        }
    }
    if let Some(max_conditions) = config.get("cond_open_max") {
        if let Ok(val) = max_conditions.parse::<usize>() {
            constants.push_str(&format!("pub const COND_OPEN_MAX: usize = {};\n", val));
        }
    }
    constants.push_str("}\n");

    // Write the generated file
    fs::write(&dest_path, constants).expect("Failed to write kernel_config.rs");

    // Inform Cargo to rerun the build script if the TOML changes
    println!("cargo:rerun-if-changed=build/kernel_config.toml");
}
