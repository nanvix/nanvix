// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::warn_with_policy;
use ::anyhow::Result;
use ::log::debug;
use ::nanvix::sandbox::{
    NAMED_RESOURCE_PREFIX,
    UNIX_SOCKET_SUFFIX,
};
use ::std::path::{
    Path,
    PathBuf,
};
use ::tokio::fs;

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
    cleanup_stale_unix_sockets(tmp_directory).await;
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
    cleanup_stale_unix_sockets(tmp_directory).await;
    cleanup_stale_files(tmp_directory).await;
}

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Removes stale Unix domain socket files left under the configured temporary directory.
///
/// # Parameters
///
/// - `tmp_directory`: Directory inspected for stale socket files.
///
async fn cleanup_stale_unix_sockets(tmp_directory: &Path) {
    let mut entries: fs::ReadDir = match fs::read_dir(tmp_directory).await {
        Ok(entries) => entries,
        Err(error) => {
            warn_with_policy!(
                "cleanup_stale_sockets(): failed to list {} (error={})",
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
                    "cleanup_stale_sockets(): failed to read tmp entry (error={})",
                    error
                );
                break;
            },
        };

        let file_name: String = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with(UNIX_SOCKET_SUFFIX) {
            continue;
        }

        let path: PathBuf = entry.path();
        let metadata: ::std::fs::Metadata = match fs::metadata(&path).await {
            Ok(meta) => meta,
            Err(error) => {
                warn_with_policy!(
                    "cleanup_stale_sockets(): failed to fetch metadata for {} (error={})",
                    path.display(),
                    error
                );
                continue;
            },
        };
        if !metadata.is_file() {
            continue;
        }

        if let Err(error) = fs::remove_file(&path).await {
            warn_with_policy!(
                "cleanup_stale_sockets(): failed to remove socket {} (error={})",
                path.display(),
                error
            );
        } else {
            debug!("cleanup_stale_sockets(): removed stale socket {}", path.display());
        }
    }
}

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
        if !file_name.starts_with(NAMED_RESOURCE_PREFIX) {
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
mod tests {
    use super::*;
    use ::anyhow::Result;
    use ::std::{
        env,
        fs,
        path::PathBuf,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    fn unique_temp_dir(prefix: &str) -> Result<PathBuf> {
        let now: SystemTime = SystemTime::now();
        let nanos: u128 = match now.duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_nanos(),
            Err(error) => {
                let reason: String =
                    format!("failed to compute monotonic timestamp for temp dir (error={error})");
                return Err(::anyhow::anyhow!(reason));
            },
        };

        let dir: PathBuf = env::temp_dir().join(format!("{prefix}-{nanos}"));
        if let Err(error) = fs::create_dir_all(&dir) {
            let reason: String =
                format!("failed to create temp dir (path={}, error={error})", dir.display());
            return Err(::anyhow::anyhow!(reason));
        }

        Ok(dir)
    }

    #[test]
    fn cleanup_stale_unix_sockets_removes_only_sockets() -> Result<()> {
        let rt: ::tokio::runtime::Runtime = ::tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| ::anyhow::anyhow!("failed to build tokio runtime: {e}"))?;
        rt.block_on(async {
            let temp_dir: PathBuf = unique_temp_dir("nvx-clean-sock")?;
            let socket_path: PathBuf = temp_dir.join(format!("test{UNIX_SOCKET_SUFFIX}"));
            let other_path: PathBuf = temp_dir.join("other.txt");

            ::tokio::fs::write(&socket_path, b"socket").await?;
            ::tokio::fs::write(&other_path, b"other").await?;

            cleanup_stale_unix_sockets(temp_dir.as_path()).await;

            let socket_exists: bool = ::tokio::fs::try_exists(&socket_path).await.unwrap_or(false);
            let other_exists: bool = ::tokio::fs::try_exists(&other_path).await.unwrap_or(false);

            assert!(!socket_exists, "stale socket should be removed");
            assert!(other_exists, "non-socket file must remain");

            let _ = fs::remove_file(&other_path);
            let _ = fs::remove_dir_all(&temp_dir);
            Ok(())
        })
    }
}
