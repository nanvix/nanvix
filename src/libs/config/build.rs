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

    // Generate Rust constants from config.
    let mut constants = String::new();
    constants.push_str("pub mod kernel {\n");
    if let Some(memory_size) = kernel_config_toml.get("memory_size") {
        if let Ok(val) = memory_size.parse::<usize>() {
            constants.push_str(&format!("pub const MEMORY_SIZE: usize = {val};\n"));
        }
    }
    if let Some(num_processors) = kernel_config_toml.get("num_processors") {
        if let Ok(val) = num_processors.parse::<usize>() {
            constants.push_str(&format!("pub const NUM_PROCESSORS: usize = {val};\n"));
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

    // Generate Rust constants from config.
    let mut constants: String = String::new();
    constants.push_str("pub mod linuxd {\n");
    if let Some(tap_name) = linuxd_config_toml.get("tap_name") {
        constants.push_str(&format!("pub const TAP_NAME: &str = \"{tap_name}\";\n"));
    }
    if let Some(guest_tap_ip) = linuxd_config_toml.get("guest_tap_ip_address") {
        constants
            .push_str(&format!("pub const GUEST_TAP_IP_ADDRESS: &str = \"{guest_tap_ip}\";\n"));
    }
    if let Some(host_tap_ip) = linuxd_config_toml.get("host_tap_ip_address") {
        constants.push_str(&format!("pub const HOST_TAP_IP_ADDRESS: &str = \"{host_tap_ip}\";\n"));
    }
    if let Some(snapshot_magic_string) = linuxd_config_toml.get("snapshot_magic_string") {
        constants.push_str(&format!(
            "pub const SNAPSHOT_MAGIC_STRING: &str = \"{snapshot_magic_string}\";\n"
        ));
    }
    if let Some(snapshot_name) = linuxd_config_toml.get("snapshot_name") {
        constants.push_str(&format!("pub const SNAPSHOT_NAME: &str = \"{snapshot_name}\";\n"));
    }
    if let Some(control_plane_port) = linuxd_config_toml.get("control_plane_port") {
        if let Ok(val) = control_plane_port.parse::<u32>() {
            constants.push_str(&format!("pub const CONTROL_PLANE_PORT: u32 = {val};\n"));
        }
    }
    if let Some(user_vm_port) = linuxd_config_toml.get("user_vm_port") {
        if let Ok(val) = user_vm_port.parse::<u32>() {
            constants.push_str(&format!("pub const USER_VM_PORT: u32 = {val};\n"));
        }
    }
    if let Some(gateway_port) = linuxd_config_toml.get("gateway_port_range_begin") {
        if let Ok(val) = gateway_port.parse::<u32>() {
            constants.push_str(&format!("pub const GATEWAY_PORT_RANGE_BEGIN: u16 = {val};\n"));
        }
    }
    if let Some(gateway_port) = linuxd_config_toml.get("gateway_port_range_end") {
        if let Ok(val) = gateway_port.parse::<u32>() {
            constants.push_str(&format!("pub const GATEWAY_PORT_RANGE_END: u16 = {val};\n"));
        }
    }
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
define_parse_hex_or_decimal!(parse_hex_or_decimal_u8, u8);

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

    // VMM shutdown command.
    let shutdown_cmd: &str = config
        .get("default_vmm_shutdown_cmd")
        .expect("default_vmm_shutdown_cmd not found in hyperlight_constants.toml");
    let shutdown_cmd_val: u8 = parse_hex_or_decimal_u8(shutdown_cmd, "default_vmm_shutdown_cmd");
    template = template.replace("{{DEFAULT_VMM_SHUTDOWN_CMD}}", &format!("{shutdown_cmd_val:#x}"));

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
    // Read the TOML file using the workspace root for a reliable path
    let manifest_dir: String =
        env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
    let workspace_dir: PathBuf = Path::new(&manifest_dir)
        .ancestors()
        .nth(3)
        .expect("Failed to find workspace root")
        .to_path_buf();
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
