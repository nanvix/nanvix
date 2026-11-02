// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
fn cleanup_stale_files_removes_only_nanvix_resources() -> Result<()> {
    let rt: ::tokio::runtime::Runtime = ::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| ::anyhow::anyhow!("failed to build tokio runtime: {e}"))?;
    rt.block_on(async {
        let temp_dir: PathBuf = unique_temp_dir("nvx-clean-sock")?;
        let socket_path: PathBuf = temp_dir.join("nvx-test.sock");
        let other_path: PathBuf = temp_dir.join("other.txt");

        ::tokio::fs::write(&socket_path, b"socket").await?;
        ::tokio::fs::write(&other_path, b"other").await?;

        cleanup_stale_files(temp_dir.as_path()).await;

        let socket_exists: bool = ::tokio::fs::try_exists(&socket_path).await.unwrap_or(false);
        let other_exists: bool = ::tokio::fs::try_exists(&other_path).await.unwrap_or(false);

        assert!(!socket_exists, "stale socket should be removed");
        assert!(other_exists, "non-socket file must remain");

        let _ = fs::remove_file(&other_path);
        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    })
}
