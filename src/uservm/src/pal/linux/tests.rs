// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::anyhow::Result;
use ::std::{
    env,
    fs,
    path::PathBuf,
    process,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

fn unique_temp_path(prefix: &str) -> Result<PathBuf> {
    let timestamp: u128 = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let pid: u32 = process::id();
    let mut path: PathBuf = env::temp_dir();
    let filename: ::std::string::String = format!("{prefix}-{pid}-{timestamp}");
    path.push(filename);
    Ok(path)
}

#[test]
fn test_mmap_reads_file_contents() -> Result<()> {
    let path: PathBuf = unique_temp_path("nanvix-filemapping-test")?;
    let content: &[u8] = b"nanvix mmap test content";
    fs::write(&path, content)?;

    let path_string: ::std::string::String = path.to_string_lossy().into_owned();
    let mapping: FileMapping = FileMapping::mmap(path_string.as_str())?;

    let expected_size: usize = content.len();
    assert_eq!(mapping.size(), expected_size, "mapping size mismatch");

    let ptr: *const u8 = mapping.ptr();
    assert!(!ptr.is_null(), "mapping pointer is null");

    let mapped: &[u8] = unsafe { ::std::slice::from_raw_parts(ptr, mapping.size()) };
    assert_eq!(mapped, content, "mapped bytes differ from file contents");

    drop(mapping);
    fs::remove_file(&path)?;

    Ok(())
}

#[test]
fn test_mmap_rejects_zero_sized_files() -> Result<()> {
    let path: PathBuf = unique_temp_path("nanvix-filemapping-empty")?;
    let _file: fs::File = fs::File::create(&path)?;

    let path_string: ::std::string::String = path.to_string_lossy().into_owned();
    let result: Result<FileMapping> = FileMapping::mmap(path_string.as_str());
    assert!(result.is_err(), "mmap should fail for zero-length files");

    fs::remove_file(&path)?;

    Ok(())
}

#[test]
fn test_file_mapping_as_slice() -> Result<()> {
    let path: PathBuf = unique_temp_path("nanvix-filemapping-as-slice")?;
    let content: &[u8] = b"as_slice test content for FileMapping";
    fs::write(&path, content)?;

    let path_string: ::std::string::String = path.to_string_lossy().into_owned();
    let mapping: FileMapping = FileMapping::mmap(path_string.as_str())?;

    let slice: &[u8] = mapping.as_slice();
    assert_eq!(slice.len(), content.len(), "slice length mismatch");
    assert_eq!(slice, content, "slice contents differ from file contents");

    drop(mapping);
    fs::remove_file(&path)?;

    Ok(())
}

#[test]
fn test_anonymous_mapping_allocates_and_drops() -> Result<()> {
    let size: usize = 4096;
    let mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

    assert!(!mapping.ptr().is_null(), "mapping pointer is null");
    assert_eq!(mapping.size(), size, "mapping size mismatch");

    // Verify the region is writable and readable.
    let data: &mut [u8] = unsafe { slice::from_raw_parts_mut(mapping.ptr(), size) };
    data[0] = 0xAB;
    data[size - 1] = 0xCD;
    assert_eq!(data[0], 0xAB, "first byte mismatch");
    assert_eq!(data[size - 1], 0xCD, "last byte mismatch");

    drop(mapping);

    Ok(())
}

#[test]
fn test_anonymous_mapping_noreserve() -> Result<()> {
    let size: usize = 4096;
    let mapping: AnonymousMapping = AnonymousMapping::new(size, true)?;

    assert!(!mapping.ptr().is_null(), "mapping pointer is null");
    assert_eq!(mapping.size(), size, "mapping size mismatch");

    drop(mapping);

    Ok(())
}

#[test]
fn test_anonymous_mapping_zero_size_fails() {
    let result: Result<AnonymousMapping> = AnonymousMapping::new(0, false);
    assert!(result.is_err(), "zero-sized anonymous mapping should fail");
}

#[test]
fn test_anonymous_mapping_as_slice() -> Result<()> {
    let size: usize = 4096;
    let mut mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

    // Write via mutable slice.
    let mutable_slice: &mut [u8] = mapping.as_mut_slice();
    mutable_slice[0] = 0x42;
    mutable_slice[size - 1] = 0x99;

    // Read via immutable slice.
    let immutable_slice: &[u8] = mapping.as_slice();
    assert_eq!(immutable_slice[0], 0x42, "first byte mismatch via as_slice");
    assert_eq!(immutable_slice[size - 1], 0x99, "last byte mismatch via as_slice");
    assert_eq!(immutable_slice.len(), size, "slice length mismatch");

    Ok(())
}

#[test]
fn test_remap_file_and_remap_anonymous() -> Result<()> {
    let path: PathBuf = unique_temp_path("nanvix-remap-file")?;
    let content: &[u8] = b"remap file test data!!!!";
    fs::write(&path, content)?;

    let size: usize = 4096;
    let mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

    // Remap the mapping to be file-backed.
    let file: fs::File = fs::File::open(&path)?;
    let fd: RawFd = ::std::os::unix::io::AsRawFd::as_raw_fd(&file);
    mapping.remap_file(fd, 0)?;

    // Verify the file contents are visible.
    let mapped: &[u8] = &mapping.as_slice()[..content.len()];
    assert_eq!(mapped, content, "file-backed remap should expose file contents");

    // Restore to anonymous.
    mapping.remap_anonymous()?;

    // After restoring, memory should be zeroed.
    let zeroed: &[u8] = mapping.as_slice();
    assert!(zeroed.iter().all(|&b| b == 0), "anonymous remap should zero memory");

    drop(mapping);
    drop(file);
    fs::remove_file(&path)?;

    Ok(())
}

#[test]
fn test_remap_file_at_sub_region() -> Result<()> {
    let path: PathBuf = unique_temp_path("nanvix-remap-at")?;
    let content: Vec<u8> = vec![0xAB; 4096];
    fs::write(&path, &content)?;

    let size: usize = 2 * 4096;
    let mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

    // Remap only the second page.
    let file: fs::File = fs::File::open(&path)?;
    let fd: RawFd = ::std::os::unix::io::AsRawFd::as_raw_fd(&file);
    mapping.remap_file_at(4096, 4096, fd, 0)?;

    // First page should still be zeroed (anonymous).
    let first_page: &[u8] = &mapping.as_slice()[..4096];
    assert!(first_page.iter().all(|&b| b == 0), "first page should remain anonymous");

    // Second page should have file contents.
    let second_page: &[u8] = &mapping.as_slice()[4096..8192];
    assert_eq!(second_page, &content[..], "second page should have file contents");

    drop(mapping);
    drop(file);
    fs::remove_file(&path)?;

    Ok(())
}

#[test]
fn test_remap_file_at_rejects_out_of_bounds() {
    let mapping: AnonymousMapping =
        AnonymousMapping::new(4096, false).expect("failed to create mapping");
    let result: Result<()> = mapping.remap_file_at(0, 8192, -1, 0);
    assert!(result.is_err(), "remap_file_at should reject out-of-bounds region");
}

#[test]
fn test_remap_file_at_rejects_zero_len() {
    let mapping: AnonymousMapping =
        AnonymousMapping::new(4096, false).expect("failed to create mapping");
    let result: Result<()> = mapping.remap_file_at(0, 0, -1, 0);
    assert!(result.is_err(), "remap_file_at should reject zero-length region");
}

#[test]
fn test_madvise_at_success() -> Result<()> {
    let size: usize = 2 * 4096;
    let mapping: AnonymousMapping = AnonymousMapping::new(size, false)?;

    // Valid madvise on a page-aligned sub-region should succeed.
    mapping.madvise_at(0, 4096, ::libc::MADV_WILLNEED)?;
    mapping.madvise_at(4096, 4096, ::libc::MADV_SEQUENTIAL)?;

    Ok(())
}

#[test]
fn test_madvise_at_zero_len_is_noop() -> Result<()> {
    let mapping: AnonymousMapping = AnonymousMapping::new(4096, false)?;

    // Zero-length madvise should return Ok without doing anything.
    mapping.madvise_at(0, 0, ::libc::MADV_WILLNEED)?;

    Ok(())
}

#[test]
fn test_madvise_at_rejects_out_of_bounds() {
    let mapping: AnonymousMapping =
        AnonymousMapping::new(4096, false).expect("failed to create mapping");
    let result: Result<()> = mapping.madvise_at(0, 8192, ::libc::MADV_WILLNEED);
    assert!(result.is_err(), "madvise_at should reject out-of-bounds region");
}

#[test]
fn test_madvise_at_rejects_unaligned_start() {
    let mapping: AnonymousMapping =
        AnonymousMapping::new(8192, false).expect("failed to create mapping");
    let result: Result<()> = mapping.madvise_at(1, 4096, ::libc::MADV_WILLNEED);
    assert!(result.is_err(), "madvise_at should reject non-page-aligned start");
}
