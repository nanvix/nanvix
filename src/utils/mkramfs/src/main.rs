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

/// Extra headroom added to the computed image size to leave free space for guest-created temporary
/// files at runtime.
const HEADROOM_FACTOR: f64 = 1.5;

/// Exit code for invalid command-line usage.
const EXIT_USAGE: i32 = 1;

/// Exit code for host filesystem I/O errors.
const EXIT_IO: i32 = 2;

/// Exit code for FAT filesystem I/O errors.
const EXIT_FAT: i32 = 3;

//==================================================================================================
// Structures
//==================================================================================================

/// A directory entry collected during the directory scan.
struct DirEntry {
    /// The full path to the directory.
    path: PathBuf,
    /// The name of the directory.
    name: String,
    /// The path relative to the source root.
    rel: PathBuf,
}

/// A file entry collected during the directory scan.
struct FileEntry {
    /// The full path to the file.
    path: PathBuf,
    /// The name of the file.
    name: String,
    /// The path relative to the source root.
    rel: PathBuf,
    /// The size of the file in bytes.
    size: u64,
}

//==================================================================================================
// Main
//==================================================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let (output, size_override, headroom, source_dir) = match parse_args(&args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("error: {msg}");
            usage(&args[0]);
            process::exit(EXIT_USAGE);
        },
    };

    // Compute image size: either user-specified or auto-calculated.
    let content_size: u64 = dir_size(&source_dir);
    let image_size: u64 = match size_override {
        Some(s) => {
            // User-specified size: still round up to page boundary.
            let page_size: u64 = arch::mem::PAGE_SIZE as u64;
            (s + page_size - 1) & !(page_size - 1)
        },
        None => {
            let factor: f64 = headroom.unwrap_or(HEADROOM_FACTOR);
            mkramfs::compute_image_size_with_factor(content_size, factor)
        },
    };

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
// Argument Parsing
//==================================================================================================

/// Creates a FAT32 image at `output` populated with the contents of `source_dir`.
fn generate_image(output: &Path, source_dir: &Path, size: u64) {
    // Create and format a zeroed FAT image.
    if let Err(e) = mkramfs::mkfatfs(output, size as usize) {
        eprintln!("mkramfs: failed to create/format FAT image: {e}");
        process::exit(EXIT_FAT);
    }

    // Populate the image with the source directory contents.
    {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(output)
            .unwrap_or_else(|e| {
                eprintln!("mkramfs: failed to open image for writing: {e}");
                process::exit(EXIT_IO);
            });
        let storage = fatfs::StdIoWrapper::new(file);
        let filesystem =
            fatfs::FileSystem::new(storage, fatfs::FsOptions::new()).unwrap_or_else(|e| {
                eprintln!("mkramfs: failed to open FAT filesystem: {e}");
                process::exit(EXIT_FAT);
            });
        let root = filesystem.root_dir();

        copy_dir_recursive(&root, source_dir, source_dir);
    }
}

/// Recursively copies the contents of `current` into the FAT directory `fat_dir`.
///
/// `base` is the original source root, used to compute relative paths for error messages.
///
/// Files are sorted by size in descending order before writing to maximize
/// contiguous cluster allocation in FAT.  This improves the hit rate of
/// the VFS `DirectReadHandle` zero-copy path.
fn copy_dir_recursive<IO, TP, OCC>(fat_dir: &fatfs::Dir<IO, TP, OCC>, current: &Path, base: &Path)
where
    IO: fatfs::ReadWriteSeek,
    TP: fatfs::TimeProvider,
    OCC: fatfs::OemCpConverter,
{
    let entries = fs::read_dir(current).unwrap_or_else(|e| {
        eprintln!("mkramfs: failed to read directory {}: {e}", current.display());
        process::exit(EXIT_IO);
    });

    // Collect and partition entries into directories and files.
    let mut dirs: Vec<DirEntry> = Vec::new();
    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries {
        let entry = entry.unwrap_or_else(|e| {
            eprintln!("mkramfs: failed to read entry in {}: {e}", current.display());
            process::exit(EXIT_IO);
        });
        let metadata = entry.metadata().unwrap_or_else(|e| {
            eprintln!("mkramfs: failed to get metadata for {}: {e}", entry.path().display());
            process::exit(EXIT_IO);
        });
        let path: PathBuf = entry.path();
        let name: String = entry.file_name().to_string_lossy().into_owned();
        let rel: PathBuf = path.strip_prefix(base).unwrap_or(&path).to_path_buf();

        if metadata.is_dir() {
            dirs.push(DirEntry { path, name, rel });
        } else if metadata.is_file() {
            files.push(FileEntry {
                path,
                name,
                rel,
                size: metadata.len(),
            });
        }
    }

    // Sort directories by relative path so traversal order is deterministic.
    dirs.sort_by(|a, b| a.rel.cmp(&b.rel));

    // Sort files largest-first so they get contiguous clusters in FAT.
    // Break ties by relative path so equal-sized files are processed in a
    // deterministic order.
    files.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.rel.cmp(&b.rel)));

    // Write files first (before subdirectories) to pack them at the
    // beginning of the FAT cluster chain.
    for file in &files {
        let data: Vec<u8> = fs::read(&file.path).unwrap_or_else(|e| {
            eprintln!("mkramfs: failed to read {}: {e}", file.rel.display());
            process::exit(EXIT_IO);
        });
        let mut fat_file = fat_dir.create_file(&file.name).unwrap_or_else(|e| {
            eprintln!("mkramfs: failed to create {}: {e:?}", file.rel.display());
            process::exit(EXIT_FAT);
        });
        fatfs::Write::write_all(&mut fat_file, &data).unwrap_or_else(|e| {
            eprintln!("mkramfs: failed to write {}: {e:?}", file.rel.display());
            process::exit(EXIT_FAT);
        });
        fatfs::Write::flush(&mut fat_file).unwrap_or_else(|e| {
            eprintln!("mkramfs: failed to flush {}: {e:?}", file.rel.display());
            process::exit(EXIT_FAT);
        });
    }

    // Then recurse into subdirectories.
    for dir in &dirs {
        fat_dir.create_dir(&dir.name).unwrap_or_else(|e| {
            eprintln!("mkramfs: failed to create directory {}: {e:?}", dir.rel.display());
            process::exit(EXIT_FAT);
        });
        let sub_dir = fat_dir.open_dir(&dir.name).unwrap_or_else(|e| {
            eprintln!("mkramfs: failed to open directory {}: {e:?}", dir.rel.display());
            process::exit(EXIT_FAT);
        });
        copy_dir_recursive(&sub_dir, &dir.path, base);
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
/// Returns `(output_path, optional_size, optional_headroom, source_dir)`.
fn parse_args(args: &[String]) -> Result<(PathBuf, Option<u64>, Option<f64>, PathBuf), String> {
    let mut output: Option<PathBuf> = None;
    let mut size: Option<u64> = None;
    let mut headroom: Option<f64> = None;
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
            "-f" => {
                i += 1;
                if i >= args.len() {
                    return Err("-f requires an argument".into());
                }
                let factor: f64 = args[i]
                    .parse()
                    .map_err(|_| format!("invalid headroom factor: {}", args[i]))?;
                if factor < 1.0 {
                    return Err("headroom factor must be at least 1.0".into());
                }
                headroom = Some(factor);
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

    Ok((output, size, headroom, source))
}

/// Prints usage information.
fn usage(program: &str) {
    eprintln!("Usage: {program} -o <output> [-s <size>] [-f <factor>] <source-dir>");
    eprintln!();
    eprintln!("Creates a FAT32 RAM filesystem image from a host directory.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -o <output>      Output image file path (required)");
    eprintln!("  -s <size>        Image size in bytes (default: auto-calculated)");
    eprintln!(
        "  -f <factor>      Headroom factor for auto-calculated size (default: {HEADROOM_FACTOR})"
    );
    eprintln!("  -h, --help       Show this help message");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {program} -o rootfs.img ./rootfs-seed/");
    eprintln!("  {program} -o rootfs.img -s 2097152 ./rootfs-seed/");
    eprintln!("  {program} -o rootfs.img -f {HEADROOM_FACTOR} ./rootfs-seed/");
}
