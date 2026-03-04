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
    process::{
        Command,
        ExitStatus,
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

    // Page size (architectural constant).
    const PAGE_SIZE: usize = 4096;

    // Extract kstack_size from config.
    let kstack_size: usize =
        parse_hex_or_decimal(required_key(&kernel_config, "kstack_size"), "kstack_size");

    // Extract kpool_base from config.
    let kpool_base: usize =
        parse_hex_or_decimal(required_key(&kernel_config, "kpool_base"), "kpool_base");

    // Extract kpool_size from config.
    let kpool_size: usize =
        parse_hex_or_decimal(required_key(&kernel_config, "kpool_size"), "kpool_size");

    // Extract kredzone_size from config.
    let kredzone_size: usize =
        parse_hex_or_decimal(required_key(&kernel_config, "kredzone_size"), "kredzone_size");

    // Extract kstack_guard_pattern from config.
    let kstack_guard_pattern: usize = parse_hex_or_decimal(
        required_key(&kernel_config, "kstack_guard_pattern"),
        "kstack_guard_pattern",
    );

    //==============================================================================================
    // Build-Time Assertions
    //==============================================================================================

    const PAGE_TABLE_SIZE: usize = 4 * 1024 * 1024;

    // kstack_size must be a multiple of the page size.
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

    // kpool_size must be a multiple of the page size.
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

    // kpool_base + kpool_size must fit within physical memory.
    let memory_size: usize =
        parse_hex_or_decimal(required_key(&kernel_config, "memory_size"), "memory_size");
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

    // kpool_base must be aligned to a page table boundary (4 MB).
    assert!(
        kpool_base.is_multiple_of(PAGE_TABLE_SIZE),
        "kpool_base ({:#x}) must be aligned to a page table boundary ({:#x})",
        kpool_base,
        PAGE_TABLE_SIZE,
    );

    // kstack_guard_pattern must fit in a 32-bit word.
    assert!(
        kstack_guard_pattern <= u32::MAX as usize,
        "kstack_guard_pattern ({:#x}) must fit in a 32-bit word",
        kstack_guard_pattern,
    );

    // Detect architecture from Cargo features.
    let is_x86_64: bool = cfg!(feature = "x86_64");

    // kredzone_size must be a multiple of the word size so that usize-indexed loads/stores
    // in kredzone.rs never silently truncate the usable slot count.
    let word_size: usize = if is_x86_64 { 8 } else { 4 };
    assert!(
        kredzone_size.is_multiple_of(word_size),
        "kredzone_size ({}) must be a multiple of the word size ({})",
        kredzone_size,
        word_size,
    );

    // Tell Cargo to rerun build script if config changes
    println!("cargo::rerun-if-changed={}", kernel_config_path.display());

    //==============================================================================================
    // Configure Toolchain
    //==============================================================================================

    let cc: String = "gcc".to_string();

    let mut cflags: Vec<String> = vec![
        "-nostdlib".to_string(),
        "-ffreestanding".to_string(),
        "-Wstack-usage=4096".to_string(),
        "-Wall".to_string(),
        "-Wextra".to_string(),
        "-Werror".to_string(),
    ];

    // Architecture-specific compiler flags.
    if is_x86_64 {
        cflags.push("-m64".to_string());
        cflags.push("-march=x86-64".to_string());
        cflags.push("-Wa,-march=generic64".to_string());
        cflags.push("-mcmodel=small".to_string());
        cflags.push("-mno-red-zone".to_string());
    } else {
        cflags.push("-m32".to_string());
        cflags.push("-march=pentiumpro".to_string());
        cflags.push("-Wa,-march=pentiumpro".to_string());
    }

    // Add defines from config for assembly constants.
    cflags.push(format!("-DKSTACK_SIZE={}", kstack_size));
    cflags.push(format!("-DKREDZONE_SIZE={}", kredzone_size));
    cflags.push(format!("-DKSTACK_GUARD_PATTERN={:#x}", kstack_guard_pattern));

    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            cflags.push("-O0".to_string());
            cflags.push("-g".to_string());
        } else {
            cflags.push("-O3".to_string());
        }
    }

    // Check for microvm feature
    cfg_if::cfg_if! {
        if #[cfg(feature = "microvm")] {
            cflags.push("-D__microvm__".to_string());
        }
        else if #[cfg(feature = "hyperlight")] {
            cflags.push("-D__hyperlight__".to_string());
        }
        else {
            cflags.push("-D__pc__".to_string());
        }
    }

    //==============================================================================================
    // Collect Assembly Source Files
    //==============================================================================================

    let arch_dir: &str = if is_x86_64 { "src/hal/arch/x86_64" } else { "src/hal/arch/x86" };
    let sources_dir: Vec<&str> = vec![arch_dir];

    // Collect *.S files in the sources directory
    let mut asm_sources = Vec::<String>::new();
    for dir in sources_dir.iter() {
        for entry in Path::new(dir).read_dir().unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension == "S" {
                    let path = path.to_str().unwrap().to_string();
                    asm_sources.push(path);
                }
            }
        }
    }

    //==============================================================================================
    // Compile Assembly Source Files
    //==============================================================================================

    // Compile assembly source files and collect object files.
    let mut object_files: Vec<String> = Vec::<String>::new();
    for asm in asm_sources.iter() {
        let obj: String =
            format!("{}/{}.o", out_dir, Path::new(asm).file_stem().unwrap().to_str().unwrap());

        let status: ExitStatus = Command::new(cc.clone())
            .args(cflags.iter().map(|s| s.as_str()).collect::<Vec<&str>>())
            .args(["-c", asm, "-o", &obj])
            .status()
            .unwrap();

        if !status.success() {
            panic!("failed to compile {asm}");
        }

        println!("cargo::rerun-if-changed={asm}");
        object_files.push(obj);
    }

    //==============================================================================================
    // Build Archive with Object Files
    //==============================================================================================

    let status: ExitStatus = Command::new("ar")
        .args(["rcs", "libkernel.a"])
        .args(&object_files)
        .current_dir(Path::new(&out_dir))
        .status()
        .unwrap();
    if !status.success() {
        panic!("failed to archive object files");
    }

    //==============================================================================================
    // Link Archive
    //==============================================================================================

    // Generate linker script from template with machine-specific MACHINE_RESERVED value.
    //
    // MACHINE_RESERVED is the space reserved after __KERNEL_END for machine-specific structures.
    // The kernel must end early enough to leave room for these structures before KPOOL_BASE.
    //
    // For Hyperlight, this includes PEB, host function definitions, and I/O buffers.
    // The exact sizes are defined in build/hyperlight_constants.toml.
    //
    // If __KERNEL_END + MACHINE_RESERVED >= KPOOL_BASE, Hyperlight creates zero-sized guard
    // pages which KVM rejects with EINVAL. We use strictly less than to ensure at least one
    // page of heap_padding exists.
    //
    // For non-Hyperlight targets, no reserved space is needed.
    let linker_dir: &str = if is_x86_64 { "build/kernel/linker/x86_64" } else { "build/kernel/linker/x86" };
    let linker_template_path: PathBuf = workspace_dir.join(format!("{}/kernel.ld.in", linker_dir));
    let linker_output_path: String = format!("{}/kernel.ld", out_dir);

    let machine_reserved: String = if cfg!(feature = "hyperlight") {
        // Read hyperlight configuration from TOML file.
        let hyperlight_config_path: PathBuf = workspace_dir.join("build/hyperlight_constants.toml");
        let hyperlight_config: HashMap<String, String> = load_toml(&hyperlight_config_path);
        println!("cargo::rerun-if-changed={}", hyperlight_config_path.display());

        let page_size_str: &str = hyperlight_config
            .get("page_size")
            .expect("page_size not found in hyperlight_constants.toml");
        let page_size: usize = page_size_str
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse page_size: '{}'", page_size_str));
        let peb_pages_str: &str = hyperlight_config
            .get("peb_pages")
            .expect("peb_pages not found in hyperlight_constants.toml");
        let peb_pages: usize = peb_pages_str
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse peb_pages: '{}'", peb_pages_str));
        let hfd_pages_str: &str = hyperlight_config
            .get("host_function_definitions_pages")
            .expect("host_function_definitions_pages not found in hyperlight_constants.toml");
        let hfd_pages: usize = hfd_pages_str.parse().unwrap_or_else(|_| {
            panic!("Failed to parse host_function_definitions_pages: '{}'", hfd_pages_str)
        });
        let input_pages_str: &str = hyperlight_config
            .get("input_data_buffer_pages")
            .expect("input_data_buffer_pages not found in hyperlight_constants.toml");
        let input_pages: usize = input_pages_str.parse().unwrap_or_else(|_| {
            panic!("Failed to parse input_data_buffer_pages: '{}'", input_pages_str)
        });
        let output_pages_str: &str = hyperlight_config
            .get("output_data_buffer_pages")
            .expect("output_data_buffer_pages not found in hyperlight_constants.toml");
        let output_pages: usize = output_pages_str.parse().unwrap_or_else(|_| {
            panic!("Failed to parse output_data_buffer_pages: '{}'", output_pages_str)
        });

        let total_pages: usize = peb_pages
            .checked_add(hfd_pages)
            .and_then(|sum| sum.checked_add(input_pages))
            .and_then(|sum| sum.checked_add(output_pages))
            .expect("Overflow calculating total reserved pages");
        let reserved: usize = total_pages
            .checked_mul(page_size)
            .expect("Overflow calculating reserved space in bytes");
        format!("{:#x}", reserved)
    } else {
        "0x0".to_string()
    };

    let linker_template: String =
        fs::read_to_string(&linker_template_path).expect("Failed to read linker script template");
    let linker_script: String = linker_template
        .replace("@MACHINE_RESERVED@", &machine_reserved)
        .replace("@KPOOL_BASE@", &format!("{:#x}", kpool_base));
    fs::write(&linker_output_path, linker_script).expect("Failed to write linker script");

    println!("cargo::rerun-if-changed={}", linker_template_path.display());
    println!("cargo::rustc-link-arg=-T{}", linker_output_path);
    println!("cargo::rustc-link-search=native={out_dir}");
    println!("cargo::rustc-link-lib=static:+whole-archive=kernel");
}
