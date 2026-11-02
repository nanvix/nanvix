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

/// Returns a unique file path in the system temp directory for test isolation.
fn unique_temp_path(suffix: &str) -> Result<(String, PathBuf)> {
    let mut path: PathBuf = env::temp_dir();
    let nanos: u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| anyhow::anyhow!("failed to compute timestamp (error={:?})", error))?
        .as_nanos();
    let file_name: String = format!("nanvix-pal-test-{}-{}-{}.tmp", process::id(), nanos, suffix);
    path.push(&file_name);
    Ok((path.to_string_lossy().into_owned(), path))
}

#[test]
fn open_returns_file_contents() -> Result<()> {
    let (path_str, path_buf): (String, PathBuf) = unique_temp_path("open")?;
    let payload: &[u8] = b"hello world";
    fs::write(&path_buf, payload)?;

    let mapping: FileMapping = FileMapping::open(&path_str)?;
    assert_eq!(mapping.size(), payload.len());

    let loaded: &[u8] = unsafe { ::std::slice::from_raw_parts(mapping.ptr(), mapping.size()) };
    assert_eq!(loaded, payload);

    // Drop the mapping before deleting the file; the section handle keeps the file open.
    drop(mapping);
    fs::remove_file(path_buf).ok();
    Ok(())
}

#[test]
fn open_as_slice_returns_file_contents() -> Result<()> {
    let (path_str, path_buf): (String, PathBuf) = unique_temp_path("as-slice")?;
    let payload: &[u8] = b"as_slice test content for FileMapping";
    fs::write(&path_buf, payload)?;

    let mapping: FileMapping = FileMapping::open(&path_str)?;

    let slice: &[u8] = mapping.as_slice();
    assert_eq!(slice.len(), payload.len(), "slice length mismatch");
    assert_eq!(slice, payload, "slice contents differ from file contents");

    drop(mapping);
    fs::remove_file(path_buf).ok();
    Ok(())
}

#[test]
fn open_returns_error_for_missing_file() {
    let result: Result<FileMapping> = FileMapping::open("/non/existent/path/to/file");
    assert!(result.is_err());
}
