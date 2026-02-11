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

    // Get CARGO_MANIFEST_DIR to find workspace root
    let manifest_dir: String = match env::var("CARGO_MANIFEST_DIR") {
        Ok(manifest_dir) => manifest_dir,
        Err(_) => panic!("failed to get CARGO_MANIFEST_DIR environment variable"),
    };
    let workspace_dir: PathBuf = Path::new(&manifest_dir)
        .ancestors()
        .nth(2) // kernel is 2 levels deep: workspace/src/kernel
        .expect("Failed to find workspace root")
        .to_path_buf();

    // Read kernel configuration
    let kernel_config_path: PathBuf = workspace_dir.join(DEFAULT_KERNEL_CONFIG_PATH);
    let kernel_config: HashMap<String, String> = load_toml(&kernel_config_path);

    // Extract kstack_size from config
    let kstack_size: usize = match kernel_config.get("kstack_size") {
        Some(size_str) => size_str
            .parse::<usize>()
            .expect("Failed to parse kstack_size"),
        None => panic!("kstack_size not found in kernel_config.toml"),
    };

    // Extract kpool_base from config
    let kpool_base_str: &str = match kernel_config.get("kpool_base") {
        Some(s) => s.as_str(),
        None => panic!("kpool_base not found in kernel_config.toml"),
    };
    let kpool_base: usize = parse_hex_or_decimal(kpool_base_str, "kpool_base");

    // Tell Cargo to rerun build script if config changes
    println!("cargo::rerun-if-changed={}", kernel_config_path.display());

    //==============================================================================================
    // Configure Toolchain
    //==============================================================================================

    let cc: String = "gcc".to_string();

    let mut cflags: Vec<String> = vec![
        "-nostdlib".to_string(),
        "-ffreestanding".to_string(),
        "-march=pentiumpro".to_string(),
        "-Wa,-march=pentiumpro".to_string(),
        "-Wstack-usage=4096".to_string(),
        "-Wall".to_string(),
        "-m32".to_string(),
        "-Wextra".to_string(),
        "-Werror".to_string(),
    ];

    // Add KSTACK_SIZE define from config
    cflags.push(format!("-DKSTACK_SIZE={}", kstack_size));

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

    let sources_dir: Vec<&str> = vec!["src/hal/arch/x86"];

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
    let linker_template_path: PathBuf = workspace_dir.join("build/kernel/linker/x86/kernel.ld.in");
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
