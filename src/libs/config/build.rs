// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! This file implements the logic to load `.toml` files in the `build/` directory so that they can
//! be reused as constants in rust source, and also in shell scripts.

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
/// Helper method to load a TOML file from a file path, and store it in a HashMap. This is a very
/// simple parser that only supports single-level TOMLs (i.e. no-nesting). Concretely, this
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

///
/// # Description
///
/// This method converts a TOML file with build-time constants for the kernel into a file with rust
/// constants that can be consumed by rust code.
///
/// # Arguments
///
/// - `kernel_config_toml_path`: Path to the TOML file to load.
/// - `kernel_config_output_path`: Path to output the rust source file.
///
fn generate_kernel_config(kernel_config_toml_path: &Path, kernel_config_output_path: &Path) {
    let kernel_config_toml: HashMap<String, String> = load_toml(kernel_config_toml_path);

    let memory_size_bytes: usize = build_utils::memory_size();

    /// Helper to retrieve a required key from the kernel config, panicking with a clear message if
    /// missing.
    fn required_key<'a>(config: &'a HashMap<String, String>, key: &str) -> &'a String {
        config
            .get(key)
            .unwrap_or_else(|| panic!("Missing required key '{}' in kernel_config.toml", key))
    }

    // Generate Rust constants from config.
    let mut constants = String::new();
    constants.push_str("pub mod kernel {\n");

    constants.push_str(&format!("pub const MEMORY_SIZE: usize = {memory_size_bytes};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "num_processors"),
        "num_processors",
    );
    constants.push_str(&format!("pub const NUM_PROCESSORS: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "kernel_watermark"),
        "kernel_watermark",
    );
    constants.push_str(&format!("pub const KERNEL_WATERMARK: usize = {val};\n"));

    let val: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "kstack_size"), "kstack_size");
    constants.push_str(&format!("pub const KSTACK_SIZE: usize = {val};\n"));

    // Stack guard watermark pattern.
    let val: u32 = parse_hex_or_decimal_u32(
        required_key(&kernel_config_toml, "kstack_guard_pattern"),
        "kstack_guard_pattern",
    );
    constants.push_str(&format!("pub const KSTACK_GUARD_PATTERN: u32 = {val:#x};\n"));

    // Kernel red zone size.
    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "kredzone_size"),
        "kredzone_size",
    );
    constants.push_str(&format!("pub const KREDZONE_SIZE: usize = {val};\n"));

    let val: u32 =
        parse_hex_or_decimal_u32(required_key(&kernel_config_toml, "timer_freq"), "timer_freq");
    constants.push_str(&format!("pub const TIMER_FREQ: u32 = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "scheduler_freq"),
        "scheduler_freq",
    );
    constants.push_str(&format!("pub const SCHEDULER_FREQ: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "max_ikc_messages"),
        "max_ikc_messages",
    );
    constants.push_str(&format!("pub const MAX_IKC_MESSAGES: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "ipc_message_size"),
        "ipc_message_size",
    );
    constants.push_str(&format!("pub const IPC_MESSAGE_SIZE: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "mutex_open_max"),
        "mutex_open_max",
    );
    constants.push_str(&format!("pub const MUTEX_OPEN_MAX: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "cond_open_max"),
        "cond_open_max",
    );
    constants.push_str(&format!("pub const COND_OPEN_MAX: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "ikc_poll_batch_size"),
        "ikc_poll_batch_size",
    );
    constants.push_str(&format!("pub const IKC_POLL_BATCH_SIZE: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "max_slab_size"),
        "max_slab_size",
    );
    constants.push_str(&format!("pub const MAX_SLAB_SIZE: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "debug_buffer_size"),
        "debug_buffer_size",
    );
    constants.push_str(&format!("pub const DEBUG_BUFFER_SIZE: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "klog_buffer_size"),
        "klog_buffer_size",
    );
    constants.push_str(&format!("pub const KLOG_BUFFER_SIZE: usize = {val};\n"));

    let val: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "max_threads"), "max_threads");
    constants.push_str(&format!("pub const MAX_THREADS: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "max_processes"),
        "max_processes",
    );
    constants.push_str(&format!("pub const MAX_PROCESSES: usize = {val};\n"));

    constants.push_str("}\n");

    //==============================================================================================
    // Build-Time Assertions
    //==============================================================================================

    let kstack_size: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "kstack_size"), "kstack_size");
    let kredzone_size: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "kredzone_size"),
        "kredzone_size",
    );

    // Architectural constants.
    const PAGE_SIZE: usize = 4096;
    const PAGE_TABLE_SIZE: usize = 4 * 1024 * 1024;
    const WORD_SIZE: usize = core::mem::size_of::<u32>();

    // kstack_size must be page-aligned.
    assert!(
        kstack_size.is_multiple_of(PAGE_SIZE),
        "kstack_size ({}) must be a multiple of PAGE_SIZE ({})",
        kstack_size,
        PAGE_SIZE,
    );

    // kstack_size must be at least two pages (one guard page + one usable page).
    assert!(
        kstack_size >= 2 * PAGE_SIZE,
        "kstack_size ({}) must be at least 2 * PAGE_SIZE ({})",
        kstack_size,
        2 * PAGE_SIZE,
    );

    // kstack_size must not exceed the size of a page table.
    assert!(
        kstack_size <= PAGE_TABLE_SIZE,
        "kstack_size ({}) must not exceed PAGE_TABLE_SIZE ({})",
        kstack_size,
        PAGE_TABLE_SIZE,
    );

    // kstack_guard_pattern is parsed as u32, so it is guaranteed to fit in a 32-bit word.

    // kredzone_size must be a multiple of the word size.
    assert!(
        kredzone_size.is_multiple_of(WORD_SIZE),
        "kredzone_size ({}) must be a multiple of the word size ({})",
        kredzone_size,
        WORD_SIZE,
    );

    // kernel_watermark must be non-zero.
    let kernel_watermark: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "kernel_watermark"),
        "kernel_watermark",
    );
    assert!(kernel_watermark > 0, "kernel_watermark must be non-zero");

    // max_threads must be at least 1 (the kernel thread always exists).
    let max_threads: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "max_threads"), "max_threads");
    assert!(max_threads >= 1, "max_threads must be at least 1");

    // max_processes must fit in a u8 so per-frame reference counts can be stored in a single byte.
    let max_processes: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "max_processes"),
        "max_processes",
    );
    assert!(max_processes >= 1, "max_processes must be at least 1");
    assert!(
        max_processes <= u8::MAX as usize,
        "max_processes ({}) must not exceed u8::MAX ({})",
        max_processes,
        u8::MAX,
    );

    // max_slab_size must be a power of two so it lines up with the kernel heap's slab tiers.
    let max_slab_size: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "max_slab_size"),
        "max_slab_size",
    );
    assert!(
        max_slab_size.is_power_of_two(),
        "max_slab_size ({}) must be a power of two",
        max_slab_size,
    );

    // Write the generated file.
    fs::write(kernel_config_output_path, constants).expect("Failed to write kernel_config.rs");
}

/// Macro to generate type-specific hex/decimal parsing functions.
///
/// This avoids code duplication while not requiring external dependencies like `num_traits`.
macro_rules! define_parse_hex_or_decimal {
    ($fn_name:ident, $type:ty) => {
        fn $fn_name(value: &str, key: &str) -> $type {
            if let Some(stripped) = value.strip_prefix("0x") {
                <$type>::from_str_radix(stripped, 16)
                    .unwrap_or_else(|_| panic!("Invalid hex value for {}: '{}'", key, value))
            } else {
                value
                    .parse()
                    .unwrap_or_else(|_| panic!("Invalid decimal value for {}: '{}'", key, value))
            }
        }
    };
}

define_parse_hex_or_decimal!(parse_hex_or_decimal_usize, usize);
define_parse_hex_or_decimal!(parse_hex_or_decimal_u32, u32);

fn main() {
    // Find the workspace root by locating the Cargo.toml with [workspace].
    let workspace_dir: PathBuf = build_utils::find_workspace_root();
    let out_dir: String = env::var("OUT_DIR").unwrap();

    // Parse kernel configuration file.
    let kernel_config_path: PathBuf = Path::new(&workspace_dir).join("build/kernel_config.toml");
    let kernel_dst_path: PathBuf = Path::new(&out_dir).join("kernel_config.rs");
    generate_kernel_config(&kernel_config_path, &kernel_dst_path);

    // Inform Cargo to rerun the build script if the TOML changes.
    println!("cargo::rerun-if-changed=build/kernel_config.toml");
    println!("cargo::rerun-if-env-changed=MEMORY_SIZE_BYTES");
}
