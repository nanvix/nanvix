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

    let val: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "memory_size"), "memory_size");
    // Hyperlight imposes a hard 1 GiB guest-memory ceiling. Fail the build if the configured
    // memory_size exceeds the hyperlight sandbox budget so the mismatch is caught immediately.
    if env::var("CARGO_FEATURE_HYPERLIGHT").is_ok() {
        const HYPERLIGHT_MEMORY_CEILING: usize = 1024 * 1024 * 1024;
        assert!(
            val <= HYPERLIGHT_MEMORY_CEILING,
            "memory_size ({val}) exceeds the Hyperlight guest-memory ceiling \
             ({HYPERLIGHT_MEMORY_CEILING}). Reduce memory_size in kernel_config.toml for \
             hyperlight builds.",
        );
    }
    constants.push_str(&format!("pub const MEMORY_SIZE: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "num_processors"),
        "num_processors",
    );
    constants.push_str(&format!("pub const NUM_PROCESSORS: usize = {val};\n"));

    let val: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "kpool_size"), "kpool_size");
    constants.push_str(&format!("pub const KPOOL_SIZE: usize = {val};\n"));

    let val: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "kpool_base"), "kpool_base");
    constants.push_str(&format!("pub const KPOOL_BASE_RAW: usize = {val:#x};\n"));

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
        required_key(&kernel_config_toml, "debug_buffer_size"),
        "debug_buffer_size",
    );
    constants.push_str(&format!("pub const DEBUG_BUFFER_SIZE: usize = {val};\n"));

    let val: usize = parse_hex_or_decimal_usize(
        required_key(&kernel_config_toml, "klog_buffer_size"),
        "klog_buffer_size",
    );
    constants.push_str(&format!("pub const KLOG_BUFFER_SIZE: usize = {val};\n"));

    constants.push_str("}\n");

    //==============================================================================================
    // Build-Time Assertions
    //==============================================================================================

    // Re-read constants needed for cross-validation.
    let memory_size: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "memory_size"), "memory_size");
    let kpool_base: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "kpool_base"), "kpool_base");
    let kpool_size: usize =
        parse_hex_or_decimal_usize(required_key(&kernel_config_toml, "kpool_size"), "kpool_size");
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

    // memory_size must accommodate the kernel pool.
    let kpool_end: usize = match kpool_base.checked_add(kpool_size) {
        Some(sum) => sum,
        None => panic!(
            "kpool_base ({:#x}) + kpool_size ({:#x}) overflows usize",
            kpool_base, kpool_size,
        ),
    };
    assert!(
        kpool_end <= memory_size,
        "kpool_base ({:#x}) + kpool_size ({:#x}) = {:#x} exceeds memory_size ({:#x})",
        kpool_base,
        kpool_size,
        kpool_end,
        memory_size,
    );

    // kpool_base must be page-table aligned (4 MB boundary).
    assert!(
        kpool_base.is_multiple_of(PAGE_TABLE_SIZE),
        "kpool_base ({:#x}) must be aligned to a page table boundary ({:#x})",
        kpool_base,
        PAGE_TABLE_SIZE,
    );

    // kpool_size must be page-aligned.
    assert!(
        kpool_size.is_multiple_of(PAGE_SIZE),
        "kpool_size ({}) must be a multiple of PAGE_SIZE ({})",
        kpool_size,
        PAGE_SIZE,
    );

    // kpool_size must not exceed the size of a page table.
    assert!(
        kpool_size <= PAGE_TABLE_SIZE,
        "kpool_size ({}) must not exceed PAGE_TABLE_SIZE ({})",
        kpool_size,
        PAGE_TABLE_SIZE,
    );

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

    // Write the generated file.
    fs::write(kernel_config_output_path, constants).expect("Failed to write kernel_config.rs");
}

///
/// # Description
///
/// This method converts a TOML file with build-time constants for linuxd into a file with rust
/// constants that can be consumed by rust code.
///
/// # Arguments
///
/// - `linuxd_config_toml_path`: Path to the TOML file to load.
/// - `linuxd_config_output_path`: Path to output the rust source file.
///
fn generate_linuxd_config(linuxd_config_toml_path: &Path, linuxd_config_output_path: &Path) {
    let linuxd_config_toml: HashMap<String, String> = load_toml(linuxd_config_toml_path);

    /// Helper to retrieve a required key from the linuxd config, panicking with a clear message if
    /// missing.
    fn required_key<'a>(config: &'a HashMap<String, String>, key: &str) -> &'a String {
        config
            .get(key)
            .unwrap_or_else(|| panic!("Missing required key '{}' in linuxd_config.toml", key))
    }

    // Generate Rust constants from config.
    let mut constants: String = String::new();
    constants.push_str("pub mod linuxd {\n");

    let tap_name: &String = required_key(&linuxd_config_toml, "tap_name");
    constants.push_str(&format!("pub const TAP_NAME: &str = \"{tap_name}\";\n"));

    let guest_tap_ip: &String = required_key(&linuxd_config_toml, "guest_tap_ip_address");
    constants.push_str(&format!("pub const GUEST_TAP_IP_ADDRESS: &str = \"{guest_tap_ip}\";\n"));

    let host_tap_ip: &String = required_key(&linuxd_config_toml, "host_tap_ip_address");
    constants.push_str(&format!("pub const HOST_TAP_IP_ADDRESS: &str = \"{host_tap_ip}\";\n"));

    let snapshot_magic_string: &String = required_key(&linuxd_config_toml, "snapshot_magic_string");
    constants.push_str(&format!(
        "pub const SNAPSHOT_MAGIC_STRING: &str = \"{snapshot_magic_string}\";\n"
    ));

    let snapshot_name: &String = required_key(&linuxd_config_toml, "snapshot_name");
    constants.push_str(&format!("pub const SNAPSHOT_NAME: &str = \"{snapshot_name}\";\n"));

    let val: u32 = parse_hex_or_decimal_u32(
        required_key(&linuxd_config_toml, "control_plane_port"),
        "control_plane_port",
    );
    constants.push_str(&format!("pub const CONTROL_PLANE_PORT: u32 = {val};\n"));

    let val: u32 =
        parse_hex_or_decimal_u32(required_key(&linuxd_config_toml, "user_vm_port"), "user_vm_port");
    constants.push_str(&format!("pub const USER_VM_PORT: u32 = {val};\n"));

    let val: u16 = parse_hex_or_decimal_u16(
        required_key(&linuxd_config_toml, "gateway_port_range_begin"),
        "gateway_port_range_begin",
    );
    constants.push_str(&format!("pub const GATEWAY_PORT_RANGE_BEGIN: u16 = {val};\n"));

    let val: u16 = parse_hex_or_decimal_u16(
        required_key(&linuxd_config_toml, "gateway_port_range_end"),
        "gateway_port_range_end",
    );
    constants.push_str(&format!("pub const GATEWAY_PORT_RANGE_END: u16 = {val};\n"));

    constants.push_str("}\n");

    // Write the generated file
    fs::write(linuxd_config_output_path, constants).expect("Failed to write linuxd_config.rs");
}

///
/// # Description
///
/// Converts a page count to a size expression string for code generation.
///
/// # Arguments
///
/// - `pages`: Number of pages.
///
/// # Returns
///
/// A string representing the size expression (e.g., "PAGE_SIZE" or "N * PAGE_SIZE").
///
fn pages_to_size_expr(pages: usize) -> String {
    assert!(pages > 0, "pages must be positive, got: {}", pages);
    if pages == 1 {
        "PAGE_SIZE".to_string()
    } else {
        format!("{pages} * PAGE_SIZE")
    }
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
define_parse_hex_or_decimal!(parse_hex_or_decimal_u16, u16);

///
/// # Description
///
/// This method converts a TOML file with build-time constants for Hyperlight into a file with rust
/// constants that can be consumed by rust code. It uses a template file with placeholders that are
/// replaced with values from the TOML configuration.
///
/// # Arguments
///
/// - `hyperlight_config_toml_path`: Path to the TOML file to load.
/// - `hyperlight_template_path`: Path to the template file.
/// - `hyperlight_config_output_path`: Path to output the rust source file.
///
fn generate_hyperlight_config(
    hyperlight_config_toml_path: &Path,
    hyperlight_template_path: &Path,
    hyperlight_config_output_path: &Path,
) {
    let config: HashMap<String, String> = load_toml(hyperlight_config_toml_path);

    // Read template file.
    let mut template: String =
        fs::read_to_string(hyperlight_template_path).expect("Failed to read hyperlight template");

    // Page size constant.
    let page_size: &str = config
        .get("page_size")
        .expect("page_size not found in hyperlight_constants.toml");
    let page_size_val: usize = page_size
        .parse()
        .expect("Failed to parse page_size as usize");
    assert!(page_size_val > 0, "page_size must be positive, got: {}", page_size_val);
    assert!(
        page_size_val.is_power_of_two(),
        "page_size must be a power of two, got: {}",
        page_size_val
    );
    template = template.replace("{{PAGE_SIZE}}", &page_size_val.to_string());

    // Boot magic.
    let boot_magic: &str = config
        .get("default_boot_magic")
        .expect("default_boot_magic not found in hyperlight_constants.toml");
    let boot_magic_val: u32 = parse_hex_or_decimal_u32(boot_magic, "default_boot_magic");
    template = template.replace("{{DEFAULT_BOOT_MAGIC}}", &format!("{boot_magic_val:#x}"));

    // Initrd base address.
    let initrd_base: &str = config
        .get("default_initrd_base")
        .expect("default_initrd_base not found in hyperlight_constants.toml");
    let initrd_base_val: usize = parse_hex_or_decimal_usize(initrd_base, "default_initrd_base");
    template = template.replace("{{DEFAULT_INITRD_BASE}}", &format!("{initrd_base_val:#x}"));

    // Initrd size bytes.
    let initrd_size_bytes: &str = config
        .get("initrd_size_bytes")
        .expect("initrd_size_bytes not found in hyperlight_constants.toml");
    let initrd_size_val: usize = initrd_size_bytes
        .parse()
        .expect("Failed to parse initrd_size_bytes as usize");
    template = template.replace("{{INITRD_SIZE_BYTES}}", &initrd_size_val.to_string());

    // PEB size (in pages -> bytes).
    let peb_pages: &str = config
        .get("peb_pages")
        .expect("peb_pages not found in hyperlight_constants.toml");
    let peb_pages_val: usize = peb_pages
        .parse()
        .expect("Failed to parse peb_pages as usize");
    assert!(peb_pages_val > 0, "peb_pages must be positive, got: {}", peb_pages_val);
    template = template.replace("{{PEB_SIZE}}", &pages_to_size_expr(peb_pages_val));

    // Host function definitions size (in pages -> bytes).
    let hfd_pages: &str = config
        .get("host_function_definitions_pages")
        .expect("host_function_definitions_pages not found in hyperlight_constants.toml");
    let hfd_pages_val: usize = hfd_pages
        .parse()
        .expect("Failed to parse host_function_definitions_pages as usize");
    assert!(
        hfd_pages_val > 0,
        "host_function_definitions_pages must be positive, got: {}",
        hfd_pages_val
    );
    template =
        template.replace("{{HOST_FUNCTION_DEFINITIONS_SIZE}}", &pages_to_size_expr(hfd_pages_val));

    // Input data buffer size (in pages -> bytes).
    let input_pages: &str = config
        .get("input_data_buffer_pages")
        .expect("input_data_buffer_pages not found in hyperlight_constants.toml");
    let input_pages_val: usize = input_pages
        .parse()
        .expect("Failed to parse input_data_buffer_pages as usize");
    assert!(
        input_pages_val > 0,
        "input_data_buffer_pages must be positive, got: {}",
        input_pages_val
    );
    template = template.replace("{{INPUT_DATA_BUFFER_SIZE}}", &pages_to_size_expr(input_pages_val));

    // Output data buffer size (in pages -> bytes).
    let output_pages: &str = config
        .get("output_data_buffer_pages")
        .expect("output_data_buffer_pages not found in hyperlight_constants.toml");
    let output_pages_val: usize = output_pages
        .parse()
        .expect("Failed to parse output_data_buffer_pages as usize");
    assert!(
        output_pages_val > 0,
        "output_data_buffer_pages must be positive, got: {}",
        output_pages_val
    );
    template =
        template.replace("{{OUTPUT_DATA_BUFFER_SIZE}}", &pages_to_size_expr(output_pages_val));

    // Stack size (in pages -> bytes)
    let stack_pages: &str = config
        .get("stack_pages")
        .expect("stack_pages not found in hyperlight_constants.toml");
    let stack_pages_val: usize = stack_pages
        .parse()
        .expect("Failed to parse stack_pages as usize");
    assert!(stack_pages_val > 0, "stack_pages must be positive, got: {}", stack_pages_val);
    template = template.replace("{{STACK_SIZE}}", &pages_to_size_expr(stack_pages_val));

    // Verify all placeholders were substituted.
    assert!(!template.contains("{{"), "Template contains unsubstituted placeholders");

    fs::write(hyperlight_config_output_path, template)
        .expect("Failed to write hyperlight_config.rs");
}

fn main() {
    // Find the workspace root by locating the Cargo.toml with [workspace].
    let workspace_dir: PathBuf = build_utils::find_workspace_root();
    let out_dir: String = env::var("OUT_DIR").unwrap();

    // Parse kernel configuration file.
    let kernel_config_path: PathBuf = Path::new(&workspace_dir).join("build/kernel_config.toml");
    let kernel_dst_path: PathBuf = Path::new(&out_dir).join("kernel_config.rs");
    generate_kernel_config(&kernel_config_path, &kernel_dst_path);

    // Parse linuxd configuration file.
    let linuxd_config_path: PathBuf = Path::new(&workspace_dir).join("build/linuxd_config.toml");
    let linuxd_dst_path: PathBuf = Path::new(&out_dir).join("linuxd_config.rs");
    generate_linuxd_config(&linuxd_config_path, &linuxd_dst_path);

    // Parse hyperlight configuration file.
    let hyperlight_config_path: PathBuf =
        Path::new(&workspace_dir).join("build/hyperlight_constants.toml");
    let hyperlight_template_path: PathBuf =
        Path::new(&workspace_dir).join("build/hyperlight_config.rs.template");
    let hyperlight_dst_path: PathBuf = Path::new(&out_dir).join("hyperlight_config.rs");
    generate_hyperlight_config(
        &hyperlight_config_path,
        &hyperlight_template_path,
        &hyperlight_dst_path,
    );

    // Inform Cargo to rerun the build script if the TOML changes.
    println!("cargo::rerun-if-changed=build/kernel_config.toml");
    println!("cargo::rerun-if-changed=build/linuxd_config.toml");
    println!("cargo::rerun-if-changed=build/hyperlight_constants.toml");
    println!("cargo::rerun-if-changed=build/hyperlight_config.rs.template");
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pages_to_size_expr_single_page() {
        let result: String = pages_to_size_expr(1);
        assert_eq!(result, "PAGE_SIZE");
    }

    #[test]
    fn test_pages_to_size_expr_multiple_pages() {
        let result: String = pages_to_size_expr(4);
        assert_eq!(result, "4 * PAGE_SIZE");
    }

    #[test]
    fn test_pages_to_size_expr_large_value() {
        let result: String = pages_to_size_expr(256);
        assert_eq!(result, "256 * PAGE_SIZE");
    }

    #[test]
    #[should_panic(expected = "pages must be positive")]
    fn test_pages_to_size_expr_zero_pages_panics() {
        pages_to_size_expr(0);
    }
}
