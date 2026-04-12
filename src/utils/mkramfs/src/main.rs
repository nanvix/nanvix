// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
    process,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Minimum image size in bytes (1 MiB).
const MIN_IMAGE_SIZE: u64 = 1024 * 1024;

/// Extra headroom added to the computed image size to leave free space for
/// guest-created temporary files at runtime.
const HEADROOM_FACTOR: f64 = 2.0;

//==================================================================================================
// Main
//==================================================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let (output, size_override, source_dir) = match parse_args(&args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("error: {msg}");
            usage(&args[0]);
            process::exit(1);
        },
    };

    // Compute image size: either user-specified or auto-calculated.
    let content_size: u64 = dir_size(&source_dir);
    let image_size: u64 = match size_override {
        Some(s) => s,
        None => {
            let computed: u64 = (content_size as f64 * HEADROOM_FACTOR) as u64;
            computed.max(MIN_IMAGE_SIZE)
        },
    };

    // Round up to page size so the guest VMM can use zero-copy file-backed mappings.
    let page_size: u64 = arch::mem::PAGE_SIZE as u64;
    let image_size: u64 = (image_size + page_size - 1) & !(page_size - 1);

    eprintln!(
        "mkramfs: source={} content={}B image={}B output={}",
        source_dir.display(),
        content_size,
        image_size,
        output.display()
    );

    generate_image(&output, &source_dir, image_size);
}

//==================================================================================================
// Image Generation
//==================================================================================================

/// Creates a FAT32 image at `output` populated with the contents of `source_dir`.
fn generate_image(output: &Path, source_dir: &Path, size: u64) {
    // Create and format a zeroed FAT image.
    mkramfs::mkfatfs(output, size as usize);

    // Populate the image with the source directory contents.
    {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(output)
            .unwrap_or_else(|e| {
                eprintln!("mkramfs: failed to open image for writing: {e}");
                process::exit(1);
            });
        let storage = fatfs::StdIoWrapper::new(file);
        let filesystem =
            fatfs::FileSystem::new(storage, fatfs::FsOptions::new()).unwrap_or_else(|e| {
                eprintln!("mkramfs: failed to open FAT filesystem: {e}");
                process::exit(1);
            });
        let root = filesystem.root_dir();

        copy_dir_recursive(&root, source_dir, source_dir);
    }
}

/// Recursively copies the contents of `current` into the FAT directory `fat_dir`.
///
/// `base` is the original source root, used to compute relative paths for error messages.
fn copy_dir_recursive<IO, TP, OCC>(fat_dir: &fatfs::Dir<IO, TP, OCC>, current: &Path, base: &Path)
where
    IO: fatfs::ReadWriteSeek,
    TP: fatfs::TimeProvider,
    OCC: fatfs::OemCpConverter,
{
    let entries = fs::read_dir(current).unwrap_or_else(|e| {
        eprintln!("mkramfs: failed to read directory {}: {e}", current.display());
        process::exit(1);
    });

    for entry in entries {
        let entry = entry.unwrap_or_else(|e| {
            eprintln!("mkramfs: failed to read entry in {}: {e}", current.display());
            process::exit(1);
        });
        let path: PathBuf = entry.path();
        let name: String = entry.file_name().to_string_lossy().into_owned();
        let rel: PathBuf = path.strip_prefix(base).unwrap_or(&path).to_path_buf();

        if path.is_dir() {
            fat_dir.create_dir(&name).unwrap_or_else(|_| {
                panic!("mkramfs: failed to create directory {}", rel.display())
            });
            let sub_dir = fat_dir
                .open_dir(&name)
                .unwrap_or_else(|_| panic!("mkramfs: failed to open directory {}", rel.display()));
            copy_dir_recursive(&sub_dir, &path, base);
        } else if path.is_file() {
            let data: Vec<u8> = fs::read(&path).unwrap_or_else(|e| {
                eprintln!("mkramfs: failed to read {}: {e}", rel.display());
                process::exit(1);
            });
            let mut fat_file = fat_dir
                .create_file(&name)
                .unwrap_or_else(|_| panic!("mkramfs: failed to create {}", rel.display()));
            fatfs::Write::write_all(&mut fat_file, &data)
                .unwrap_or_else(|_| panic!("mkramfs: failed to write {}", rel.display()));
            fatfs::Write::flush(&mut fat_file)
                .unwrap_or_else(|_| panic!("mkramfs: failed to flush {}", rel.display()));
        }
    }
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Computes the total size of all files under `dir` (recursive).
fn dir_size(dir: &Path) -> u64 {
    let mut total: u64 = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path: PathBuf = entry.path();
            if path.is_dir() {
                total += dir_size(&path);
            } else if path.is_file() {
                total += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

/// Parses command-line arguments.
///
/// Returns `(output_path, optional_size, source_dir)`.
fn parse_args(args: &[String]) -> Result<(PathBuf, Option<u64>, PathBuf), String> {
    let mut output: Option<PathBuf> = None;
    let mut size: Option<u64> = None;
    let mut source: Option<PathBuf> = None;
    let mut i: usize = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                if i >= args.len() {
                    return Err("-o requires an argument".into());
                }
                output = Some(PathBuf::from(&args[i]));
            },
            "-s" => {
                i += 1;
                if i >= args.len() {
                    return Err("-s requires an argument".into());
                }
                let bytes: u64 = args[i]
                    .parse()
                    .map_err(|_| format!("invalid size: {}", args[i]))?;
                if bytes < MIN_IMAGE_SIZE {
                    return Err(format!("image size must be at least {} bytes", MIN_IMAGE_SIZE));
                }
                size = Some(bytes);
            },
            "-h" | "--help" => {
                return Err("help requested".into());
            },
            arg if arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}"));
            },
            _ => {
                if source.is_some() {
                    return Err("only one source directory allowed".into());
                }
                source = Some(PathBuf::from(&args[i]));
            },
        }
        i += 1;
    }

    let output: PathBuf = output.ok_or("-o <output> is required")?;
    let source: PathBuf = source.ok_or("<source-dir> is required")?;

    if !source.is_dir() {
        return Err(format!("{} is not a directory", source.display()));
    }

    Ok((output, size, source))
}

/// Prints usage information.
fn usage(program: &str) {
    eprintln!("Usage: {program} -o <output> [-s <size>] <source-dir>");
    eprintln!();
    eprintln!("Creates a FAT32 RAM filesystem image from a host directory.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -o <output>      Output image file path (required)");
    eprintln!("  -s <size>        Image size in bytes (default: auto-calculated)");
    eprintln!("  -h, --help       Show this help message");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {program} -o rootfs.img ./rootfs-seed/");
    eprintln!("  {program} -o rootfs.img -s 2097152 ./rootfs-seed/");
}
