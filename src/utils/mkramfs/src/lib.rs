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
    fs,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Minimum image size in bytes (1 MiB).
pub const MIN_IMAGE_SIZE: u64 = 1024 * 1024;

/// Default headroom factor applied to content size when computing the image size.
pub const HEADROOM_FACTOR: f64 = 1.5;

/// Page size used for alignment (must match the target architecture page size).
const PAGE_SIZE: u64 = ::arch::mem::PAGE_SIZE as u64;

//==================================================================================================
// Public API
//==================================================================================================

/// Computes a page-aligned image size from the given content size.
///
/// Applies [`HEADROOM_FACTOR`] to the content size, clamps to [`MIN_IMAGE_SIZE`], and rounds
/// up to the next page boundary so that the resulting file is suitable for zero-copy
/// file-backed mappings.
pub fn compute_image_size(content_size: u64) -> u64 {
    compute_image_size_with_factor(content_size, HEADROOM_FACTOR)
}

/// Like [`compute_image_size`] but with a caller-specified headroom factor.
pub fn compute_image_size_with_factor(content_size: u64, factor: f64) -> u64 {
    let raw: u64 = if content_size == 0 {
        MIN_IMAGE_SIZE
    } else {
        let computed: u64 = (content_size as f64 * factor) as u64;
        computed.max(MIN_IMAGE_SIZE)
    };
    page_align(raw)
}

/// Rounds `value` up to the next page boundary.
fn page_align(value: u64) -> u64 {
    let mask: u64 = PAGE_SIZE - 1;
    (value + mask) & !mask
}

/// Creates a zeroed file of `size` bytes at `path` and formats it as a FAT filesystem.
///
/// # Errors
///
/// Returns an error if the file cannot be created, opened, or formatted.
pub fn mkfatfs(path: &Path, size: usize) -> std::io::Result<()> {
    let buf: Vec<u8> = vec![0u8; size];
    std::fs::write(path, &buf)?;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;
    let mut storage = fatfs::StdIoWrapper::new(file);
    fatfs::format_volume(&mut storage, fatfs::FormatVolumeOptions::new())
        .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
    Ok(())
}

/// Creates a FAT32 image at `output` populated with the contents of `source_dir`.
///
/// # Errors
///
/// Returns an error if the image cannot be created, formatted, or populated.
pub fn generate_image(output: &Path, source_dir: &Path, size: u64) -> std::io::Result<()> {
    mkfatfs(output, size as usize)?;

    let file = fs::OpenOptions::new().read(true).write(true).open(output)?;
    let storage = fatfs::StdIoWrapper::new(file);
    let filesystem = fatfs::FileSystem::new(storage, fatfs::FsOptions::new())
        .map_err(|e| std::io::Error::other(format!("{e:?}")))?;
    let root = filesystem.root_dir();

    copy_dir_recursive(&root, source_dir, source_dir)?;
    Ok(())
}

/// Computes the total size of all files under `dir` (recursive).
pub fn dir_size(dir: &Path) -> u64 {
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

/// Recursively copies the contents of `current` into the FAT directory `fat_dir`.
///
/// `base` is the original source root, used to compute relative paths for error messages.
///
/// # Errors
///
/// Returns an error if any directory or file operation fails.
pub fn copy_dir_recursive<IO, TP, OCC>(
    fat_dir: &fatfs::Dir<IO, TP, OCC>,
    current: &Path,
    base: &Path,
) -> std::io::Result<()>
where
    IO: fatfs::ReadWriteSeek,
    TP: fatfs::TimeProvider,
    OCC: fatfs::OemCpConverter,
{
    let entries = fs::read_dir(current)?;

    for entry in entries {
        let entry = entry?;
        let path: PathBuf = entry.path();
        let name: String = entry.file_name().to_string_lossy().into_owned();
        let rel: PathBuf = path.strip_prefix(base).unwrap_or(&path).to_path_buf();

        if path.is_dir() {
            fat_dir.create_dir(&name).map_err(|e| {
                std::io::Error::other(format!(
                    "failed to create directory {}: {e:?}",
                    rel.display()
                ))
            })?;
            let sub_dir = fat_dir.open_dir(&name).map_err(|e| {
                std::io::Error::other(format!("failed to open directory {}: {e:?}", rel.display()))
            })?;
            copy_dir_recursive(&sub_dir, &path, base)?;
        } else if path.is_file() {
            let data: Vec<u8> = fs::read(&path)?;
            let mut fat_file = fat_dir.create_file(&name).map_err(|e| {
                std::io::Error::other(format!("failed to create {}: {e:?}", rel.display()))
            })?;
            fatfs::Write::write_all(&mut fat_file, &data).map_err(|e| {
                std::io::Error::other(format!("failed to write {}: {e:?}", rel.display()))
            })?;
            fatfs::Write::flush(&mut fat_file).map_err(|e| {
                std::io::Error::other(format!("failed to flush {}: {e:?}", rel.display()))
            })?;
        }
    }
    Ok(())
}
