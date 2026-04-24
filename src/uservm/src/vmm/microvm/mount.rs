// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host-directory mount support for standalone mode.
//!
//! This module provides helpers that:
//! 1. Build a FAT32 image from a host directory using `mkramfs`.
//! 2. Combine an optional root RAMFS and the mount image into a multi-image container.
//! 3. After VM shutdown, extract modified files from guest memory back to the host directory.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    info,
    trace,
};
use ::multiimage::{
    self,
    ImageDescriptor,
    ImageEntry,
    MOUNTFS_MMIO_TAG,
    MultiImageLayout,
    ROOTFS_MMIO_TAG,
};
use ::std::path::{
    Path,
    PathBuf,
};
use ::tempfile::TempPath;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum mount image size: 1/8 of guest MEMORY_SIZE.
const MAX_MOUNT_IMAGE_SIZE: usize = ::config::kernel::MEMORY_SIZE / 8;

//==================================================================================================
// Public API
//==================================================================================================

///
/// # Description
///
/// Builds a FAT32 image from the contents of `host_dir`. If the directory is empty, creates an
/// empty formatted FAT image of [`mkramfs::MIN_IMAGE_SIZE`] bytes.
///
/// The resulting image is written to a temporary file wrapped in a [`TempPath`] that
/// automatically deletes the file when dropped, ensuring cleanup on all code paths
/// (including errors).
///
/// # Parameters
///
/// - `host_dir`: Path to the host directory whose contents should be packaged.
///
/// # Returns
///
/// On success, returns the [`TempPath`] to the generated image file and its size in bytes.
///
pub fn build_mount_image(host_dir: &Path) -> Result<(TempPath, usize)> {
    trace!("build_mount_image(): host_dir={host_dir:?}");

    let content_size: u64 = mkramfs::dir_size(host_dir);
    let image_size: u64 = mkramfs::compute_image_size(content_size);

    // Enforce the 1/8 MEMORY_SIZE limit.
    let image_size: u64 = image_size.min(MAX_MOUNT_IMAGE_SIZE as u64);

    if content_size > image_size {
        let reason: String = format!(
            "host directory content ({content_size} bytes) exceeds mount image limit \
             ({MAX_MOUNT_IMAGE_SIZE} bytes)"
        );
        error!("build_mount_image(): {reason}");
        anyhow::bail!(reason);
    }

    // Create a named temporary file that is automatically removed on drop.
    let tmp_file: tempfile::NamedTempFile = tempfile::Builder::new()
        .prefix("nanvix-mount-")
        .suffix(".img")
        .tempfile()
        .map_err(|e| {
            let reason: String = format!("failed to create temporary mount image file: {e}");
            error!("build_mount_image(): {reason}");
            anyhow::anyhow!(reason)
        })?;
    let output: TempPath = tmp_file.into_temp_path();

    if content_size == 0 {
        info!(
            "build_mount_image(): empty directory, creating empty FAT image ({image_size} bytes)"
        );
        mkramfs::mkfatfs(&output, usize::try_from(image_size).unwrap_or(usize::MAX)).map_err(
            |e| {
                let reason: String = format!("failed to create FAT image: {e}");
                error!("build_mount_image(): {reason}");
                anyhow::anyhow!(reason)
            },
        )?;
    } else {
        info!(
            "build_mount_image(): packaging {content_size} bytes into {image_size}-byte FAT image"
        );
        mkramfs::generate_image(&output, host_dir, image_size).map_err(|e| {
            let reason: String = format!("failed to generate FAT image: {e}");
            error!("build_mount_image(): {reason}");
            anyhow::anyhow!(reason)
        })?;
    }

    let actual_size: usize = std::fs::metadata(output.as_ref() as &Path)
        .map(|m| usize::try_from(m.len()).unwrap_or(usize::MAX))
        .map_err(|e| {
            let reason: String = format!("failed to stat mount image at {output:?}: {e}");
            error!("build_mount_image(): {reason}");
            anyhow::anyhow!(reason)
        })?;

    info!("build_mount_image(): image ready at {output:?} ({actual_size} bytes)");
    Ok((output, actual_size))
}

///
/// # Description
///
/// Computes a multi-image container layout that combines an optional root RAMFS image and a
/// mount image. Instead of writing a concatenated file to disk, this returns a
/// [`MultiImageLayout`] descriptor that the VMM backends can use to map each sub-image file
/// directly into guest memory (zero-copy).
///
/// # Parameters
///
/// - `rootfs_path`: Optional path to the root RAMFS image (from `-ramfs` flag).
/// - `mountfs_path`: Path to the mount image (from [`build_mount_image`]).
///
/// # Returns
///
/// On success, returns the computed [`MultiImageLayout`].
///
pub fn compute_unified_layout(
    rootfs_path: Option<&Path>,
    mountfs_path: &Path,
) -> Result<MultiImageLayout> {
    trace!("compute_unified_layout(): rootfs={rootfs_path:?}, mountfs={mountfs_path:?}");

    let mut descriptors: Vec<ImageDescriptor<'_>> = Vec::new();

    if let Some(rootfs) = rootfs_path {
        descriptors.push(ImageDescriptor {
            tag: ROOTFS_MMIO_TAG,
            path: rootfs,
            flags: multiimage::FLAG_READONLY,
        });
    }

    descriptors.push(ImageDescriptor {
        tag: MOUNTFS_MMIO_TAG,
        path: mountfs_path,
        flags: 0,
    });

    let layout: MultiImageLayout =
        multiimage::compute_multiimage_layout(&descriptors).map_err(|e| {
            let reason: String = format!("failed to compute unified image layout: {e:?}");
            error!("compute_unified_layout(): {reason}");
            anyhow::anyhow!(reason)
        })?;

    info!(
        "compute_unified_layout(): layout ready ({} regions, total_size={} bytes)",
        layout.regions.len(),
        layout.total_size
    );

    Ok(layout)
}

///
/// # Description
///
/// Extracts the MOUNTFS sub-image from guest memory and copies its contents back to `host_dir`.
///
/// This is called after the guest VM shuts down. The function:
/// 1. Parses the multi-image header from the RAMFS region in guest memory.
/// 2. Locates the MOUNTFS entry.
/// 3. Reads the MOUNTFS sub-image data.
/// 4. Opens it as a FAT filesystem.
/// 5. Copies all files back to `host_dir`, including those created or modified by the guest.
///
/// # Parameters
///
/// - `ramfs_data`: Slice of guest memory covering the entire RAMFS/multi-image region.
/// - `host_dir`: Path to the original host directory that was mounted.
///
pub fn copyback_mount_image(ramfs_data: &[u8], host_dir: &Path) -> Result<()> {
    trace!("copyback_mount_image(): host_dir={host_dir:?}, region_size={}", ramfs_data.len());

    if !multiimage::is_multiimage(ramfs_data) {
        let reason: String = "RAMFS region does not contain a multi-image container".to_string();
        error!("copyback_mount_image(): {reason}");
        anyhow::bail!(reason);
    }

    let header: multiimage::MultiImageHeader =
        multiimage::parse_header(ramfs_data).map_err(|e| {
            let reason: String = format!("failed to parse multi-image header: {e:?}");
            error!("copyback_mount_image(): {reason}");
            anyhow::anyhow!(reason)
        })?;

    let entries: &[ImageEntry] =
        multiimage::parse_entries(ramfs_data, header.num_images).map_err(|e| {
            let reason: String = format!("failed to parse multi-image entries: {e:?}");
            error!("copyback_mount_image(): {reason}");
            anyhow::anyhow!(reason)
        })?;

    let mountfs: &ImageEntry = multiimage::find_entry_by_tag(entries, &MOUNTFS_MMIO_TAG)
        .ok_or_else(|| {
            let reason: String = "MOUNTFS entry not found in multi-image container".to_string();
            error!("copyback_mount_image(): {reason}");
            anyhow::anyhow!(reason)
        })?;

    let offset: usize = usize::try_from(mountfs.offset)
        .map_err(|_| anyhow::anyhow!("MOUNTFS offset exceeds platform address space"))?;
    let size: usize = usize::try_from(mountfs.size)
        .map_err(|_| anyhow::anyhow!("MOUNTFS size exceeds platform address space"))?;

    if offset + size > ramfs_data.len() {
        let reason: String = format!(
            "MOUNTFS region out of bounds (offset={offset}, size={size}, total={})",
            ramfs_data.len()
        );
        error!("copyback_mount_image(): {reason}");
        anyhow::bail!(reason);
    }

    let mountfs_data: &[u8] = &ramfs_data[offset..offset + size];

    // Write the MOUNTFS data to a temporary file so we can open it with fatfs.
    let tmp_path: PathBuf =
        std::env::temp_dir().join(format!("nanvix-copyback-{}.img", std::process::id()));
    std::fs::write(&tmp_path, mountfs_data).map_err(|e| {
        let reason: String = format!("failed to write copyback image to {tmp_path:?}: {e}");
        error!("copyback_mount_image(): {reason}");
        anyhow::anyhow!(reason)
    })?;

    // Open the FAT image and extract files.
    let file: std::fs::File = std::fs::OpenOptions::new()
        .read(true)
        .open(&tmp_path)
        .map_err(|e| {
            let reason: String = format!("failed to open copyback image: {e}");
            error!("copyback_mount_image(): {reason}");
            anyhow::anyhow!(reason)
        })?;

    let storage = fatfs::StdIoWrapper::new(file);
    let fs = fatfs::FileSystem::new(storage, fatfs::FsOptions::new()).map_err(|e| {
        let reason: String = format!("failed to open FAT filesystem for copyback: {e}");
        error!("copyback_mount_image(): {reason}");
        anyhow::anyhow!(reason)
    })?;

    // Extract all files from the FAT image into the host directory, creating directories
    // as needed and overwriting matching files, but leaving unrelated existing files in place.
    extract_dir_recursive(&fs.root_dir(), host_dir)?;

    // Clean up temporary file.
    let _ = std::fs::remove_file(&tmp_path);

    info!("copyback_mount_image(): files copied back to {host_dir:?}");
    Ok(())
}

//==================================================================================================
// Private Helpers
//==================================================================================================

/// Recursively extracts all files and directories from a FAT directory into `host_dir`.
fn extract_dir_recursive<IO, TP, OCC>(
    fat_dir: &fatfs::Dir<IO, TP, OCC>,
    host_dir: &Path,
) -> Result<()>
where
    IO: fatfs::ReadWriteSeek,
    TP: fatfs::TimeProvider,
    OCC: fatfs::OemCpConverter,
{
    // Ensure the host directory exists.
    std::fs::create_dir_all(host_dir)
        .map_err(|e| anyhow::anyhow!("failed to create directory {}: {e}", host_dir.display()))?;

    for entry in fat_dir.iter() {
        let entry =
            entry.map_err(|e| anyhow::anyhow!("failed to read FAT directory entry: {e:?}"))?;

        let name: String = entry.file_name();

        // Skip the dot entries.
        if name == "." || name == ".." {
            continue;
        }

        let target: PathBuf = host_dir.join(&name);

        if entry.is_dir() {
            extract_dir_recursive(&entry.to_dir(), &target)?;
        } else {
            let mut data: Vec<u8> = Vec::new();
            let mut file = entry.to_file();
            let mut buf: [u8; 4096] = [0u8; 4096];
            loop {
                let n: usize = fatfs::Read::read(&mut file, &mut buf)
                    .map_err(|e| anyhow::anyhow!("failed to read FAT file {name}: {e:?}"))?;
                if n == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..n]);
            }
            std::fs::write(&target, &data)
                .map_err(|e| anyhow::anyhow!("failed to write {}: {e}", target.display()))?;
        }
    }

    Ok(())
}
