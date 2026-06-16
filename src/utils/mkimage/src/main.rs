// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use std::{
    fs,
    process,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Program name.
const PROGRAM_NAME: &str = "mkimage";

//==================================================================================================
// Main
//==================================================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag before validating argument count.
    if args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
        process::exit(0);
    }

    if args.len() < 4 {
        usage();
        process::exit(1);
    }

    // Parse -o <output> and -k <kernel_args> flags.
    let mut output_path: Option<&str> = None;
    let mut kernel_args: Option<&str> = None;
    let mut entry_args: Vec<&str> = Vec::new();
    let mut i: usize = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                if i + 1 >= args.len() {
                    eprintln!("{}: error: -o requires an output path", PROGRAM_NAME);
                    process::exit(1);
                }
                output_path = Some(&args[i + 1]);
                i += 2;
            },
            "-k" => {
                if i + 1 >= args.len() {
                    eprintln!("{}: error: -k requires a kernel arguments string", PROGRAM_NAME);
                    process::exit(1);
                }
                kernel_args = Some(&args[i + 1]);
                i += 2;
            },
            "-h" | "--help" => {
                usage();
                process::exit(0);
            },
            _ => {
                entry_args.push(&args[i]);
                i += 1;
            },
        }
    }

    let output_path: &str = match output_path {
        Some(path) => path,
        None => {
            eprintln!("{}: error: -o <output> is required", PROGRAM_NAME);
            usage();
            process::exit(1);
        },
    };

    if entry_args.is_empty() {
        eprintln!("{}: error: at least one binary entry is required", PROGRAM_NAME);
        usage();
        process::exit(1);
    }

    // Build the multibinary image.
    let mut builder: multibin::builder::MultibinBuilder = multibin::builder::MultibinBuilder::new();

    for entry_arg in &entry_args {
        // Format: path/to/binary.elf;cmdline
        let (elf_path, cmdline): (&str, &str) = match entry_arg.split_once(';') {
            Some((path, cmd)) => (path, cmd),
            None => {
                eprintln!(
                    "{}: error: invalid entry '{}' (expected 'path.elf;cmdline')",
                    PROGRAM_NAME, entry_arg
                );
                process::exit(1);
            },
        };

        let elf_data: Vec<u8> = match fs::read(elf_path) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("{}: error: failed to read '{}': {}", PROGRAM_NAME, elf_path, err);
                process::exit(1);
            },
        };

        eprintln!(
            "{}: adding '{}' ({} bytes, cmdline='{}')",
            PROGRAM_NAME,
            elf_path,
            elf_data.len(),
            cmdline
        );
        if let Err(err) = builder.add(elf_data, cmdline) {
            eprintln!("{}: error: {:?}", PROGRAM_NAME, err);
            process::exit(1);
        }
    }

    // Set kernel arguments if provided.
    if let Some(kargs) = kernel_args {
        eprintln!("{}: kernel args: '{}'", PROGRAM_NAME, kargs);
        builder.set_kernel_args(kargs);
    }

    let image: Vec<u8> = match builder.build() {
        Ok(image) => image,
        Err(err) => {
            eprintln!("{}: error: failed to build image: {:?}", PROGRAM_NAME, err);
            process::exit(1);
        },
    };

    match fs::write(output_path, &image) {
        Ok(()) => {
            eprintln!(
                "{}: wrote '{}' ({} bytes, {} entries)",
                PROGRAM_NAME,
                output_path,
                image.len(),
                entry_args.len()
            );
        },
        Err(err) => {
            eprintln!("{}: error: failed to write '{}': {}", PROGRAM_NAME, output_path, err);
            process::exit(1);
        },
    }
}

///
/// # Description
///
/// Prints usage information.
///
fn usage() {
    eprintln!(
        "Usage: {} -o <output.img> [-k <kernel_args>] <binary.elf;cmdline> [<binary.elf;cmdline> \
         ...]",
        PROGRAM_NAME
    );
    eprintln!();
    eprintln!("Creates a Nanvix multibinary image from individual ELF binaries.");
    eprintln!();
    eprintln!("Each entry is specified as 'path/to/binary.elf;cmdline' where the");
    eprintln!("semicolon separates the file path from the command line string.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -o <output>       Output image file path (required).");
    eprintln!("  -k <kernel_args>  Kernel arguments shared by all binaries in the image.");
    eprintln!();
    eprintln!("Example:");
    eprintln!(
        "  {} -o nanvix.img -k snapshot procd.elf;procd memd.elf;memd testd.elf;testd",
        PROGRAM_NAME
    );
}
