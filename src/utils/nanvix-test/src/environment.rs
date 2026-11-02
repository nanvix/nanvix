// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::warn_with_policy;
use ::anyhow::Result;
use ::log::debug;
use ::std::path::{
    Path,
    PathBuf,
};
use ::tokio::fs;

/// Prefix used by Nanvix host resources in temporary directories.
const NANVIX_RESOURCE_PREFIX: &str = "nvx";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sanitizes the host before launching Nanvix Daemon runs by deleting stale sockets and files.
///
/// # Parameters
///
/// - `tmp_directory`: Directory inspected for stale Nanvix artifacts.
///
pub(crate) async fn prepare_runner_environment(tmp_directory: &Path) {
    cleanup_stale_files(tmp_directory).await;
}

///
/// # Description
///
/// Cleans stale artifacts left after a Nanvix Daemon run, mirroring the teardown logic from the
/// reference shell runner.
///
/// # Parameters
///
/// - `tmp_directory`: Directory inspected for stale Nanvix artifacts.
///
pub(crate) async fn cleanup_after_run(tmp_directory: &Path) {
    cleanup_stale_files(tmp_directory).await;
}

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Removes stale files and directories under the configured temporary directory that follow the
/// `NAMED_RESOURCE_PREFIX` convention used by sandboxes.
///
/// # Parameters
///
/// - `tmp_directory`: Directory inspected for stale Nanvix artifacts.
///
async fn cleanup_stale_files(tmp_directory: &Path) {
    let mut entries: fs::ReadDir = match fs::read_dir(tmp_directory).await {
        Ok(entries) => entries,
        Err(error) => {
            warn_with_policy!(
                "cleanup_stale_named_resources(): failed to list {} (error={})",
                tmp_directory.display(),
                error
            );
            return;
        },
    };

    loop {
        let entry: fs::DirEntry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(error) => {
                warn_with_policy!(
                    "cleanup_stale_named_resources(): failed to read tmp entry (error={})",
                    error
                );
                break;
            },
        };

        let file_name: String = entry.file_name().to_string_lossy().to_string();
        if !file_name.starts_with(NANVIX_RESOURCE_PREFIX) {
            continue;
        }

        let path: PathBuf = entry.path();
        let metadata: ::std::fs::Metadata = match fs::metadata(&path).await {
            Ok(meta) => meta,
            Err(error) => {
                warn_with_policy!(
                    "cleanup_stale_named_resources(): failed to fetch metadata for {} (error={})",
                    path.display(),
                    error
                );
                continue;
            },
        };

        let removal_result: Result<(), ::std::io::Error> = if metadata.is_dir() {
            fs::remove_dir_all(&path).await
        } else {
            fs::remove_file(&path).await
        };

        match removal_result {
            Ok(()) => {
                debug!(
                    "cleanup_stale_named_resources(): removed stale resource {}",
                    path.display()
                );
            },
            Err(error) => {
                warn_with_policy!(
                    "cleanup_stale_named_resources(): failed to remove {} (error={})",
                    path.display(),
                    error
                );
            },
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests;
