// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::warn_with_policy;
use ::anyhow::Result;
use ::log::{
    debug,
    info,
};
use ::nanvix::{
    config::linuxd::SNAPSHOT_NAME,
    sandbox::{
        NAMED_RESOURCE_PREFIX,
        NETNS_NAME_PREFIX,
        UNIX_SOCKET_SUFFIX,
        VETH_HOST_PREFIX,
    },
};
use ::nanvixd::config::{
    DEFAULT_L2_SNAPSHOT_DIRECTORY,
    DEFAULT_SNAPSHOT_FILE_NAME,
};
use ::std::path::{
    Path,
    PathBuf,
};
use ::tokio::{
    fs,
    process::Command,
    time::{
        Duration,
        Instant,
        sleep,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sanitizes the host before launching Nanvix Daemon runs by deleting stale sockets and, when L2
/// mode is enabled, removing stale network namespaces and waiting for TCP TIME_WAIT sockets.
///
/// # Parameters
///
/// - `l2_enabled`: Indicates whether the upcoming run requires L2 deployment.
/// - `port_num`: TCP port exposed by the Nanvix Daemon HTTP endpoint.
/// - `tmp_directory`: Directory inspected for stale Nanvix artifacts.
/// - `tcp_cleanup_max_wait_seconds`: Maximum seconds spent waiting for lingering TIME_WAIT
///   sockets.
/// - `tcp_cleanup_poll_interval_seconds`: Seconds between TIME_WAIT socket inspections.
///
pub(crate) async fn prepare_runner_environment(
    l2_enabled: bool,
    port_num: u16,
    tmp_directory: &Path,
    tcp_cleanup_max_wait_seconds: u64,
    tcp_cleanup_poll_interval_seconds: u64,
) {
    cleanup_stale_unix_sockets(tmp_directory).await;
    cleanup_stale_files(tmp_directory).await;
    if l2_enabled {
        cleanup_stale_netns().await;
        wait_for_tcp_cleanup(
            port_num,
            tcp_cleanup_max_wait_seconds,
            tcp_cleanup_poll_interval_seconds,
        )
        .await;
    }
}

///
/// # Description
///
/// Guarantees the artifacts required for L2 deployments (initramfs and snapshot) are available by
/// invoking the same helper scripts used by the Makefile targets.
///
/// # Parameters
///
/// - `toolchain_path`: Absolute path to the Nanvix toolchain directory.
/// - `working_directory`: Root of the Nanvix repository (used to locate scripts and images).
///
/// # Return Value
///
/// Returns `Ok(())` when the artifacts exist (either prior to or after script execution);
/// otherwise returns an error that surfaces the failing script or missing artifact.
///
/// # Errors
///
/// Returns an error when the helper scripts are missing, fail to execute, or the artifacts remain
/// absent after generation attempts.
///
pub(crate) async fn prepare_l2_artifacts(
    toolchain_path: &str,
    working_directory: &Path,
) -> Result<()> {
    let images_dir: PathBuf = working_directory.join(DEFAULT_L2_SNAPSHOT_DIRECTORY);
    if let Err(error) = fs::create_dir_all(&images_dir).await {
        let reason: String = format!(
            "failed to create images directory (path={}, error={error})",
            images_dir.display()
        );
        warn_with_policy!("prepare_l2_artifacts(): {reason}");
        return Err(::anyhow::anyhow!(reason));
    }

    let snapshot_path: PathBuf = images_dir.join(SNAPSHOT_NAME);
    let initramfs_path: PathBuf = images_dir.join(DEFAULT_SNAPSHOT_FILE_NAME);

    // Check for existing artifacts.
    let snapshot_exists: bool = match fs::try_exists(&snapshot_path).await {
        Ok(exists) => exists,
        Err(error) => {
            warn_with_policy!(
                "prepare_l2_artifacts(): failed to check snapshot existence (path={}, error={})",
                snapshot_path.display(),
                error
            );
            false
        },
    };
    let initramfs_exists: bool = match fs::try_exists(&initramfs_path).await {
        Ok(exists) => exists,
        Err(error) => {
            warn_with_policy!(
                "prepare_l2_artifacts(): failed to check initramfs existence (path={}, error={})",
                initramfs_path.display(),
                error
            );
            false
        },
    };

    if snapshot_exists && initramfs_exists {
        debug!(
            "prepare_l2_artifacts(): reusing existing L2 artifacts (snapshot={}, initramfs={})",
            snapshot_path.display(),
            initramfs_path.display()
        );
        return Ok(());
    }

    let scripts_dir: PathBuf = working_directory.join("scripts");
    let initramfs_script: PathBuf = scripts_dir.join("generate-l2-initramfs.sh");
    let snapshot_script: PathBuf = scripts_dir.join("generate-l2-snapshot.sh");

    info!(
        "prepare_l2_artifacts(): generating L2 initramfs (script={}, output={})",
        initramfs_script.display(),
        initramfs_path.display()
    );
    run_script(&initramfs_script, working_directory, &[]).await?;

    info!(
        "prepare_l2_artifacts(): generating L2 snapshot (script={}, output={}, toolchain={})",
        snapshot_script.display(),
        snapshot_path.display(),
        toolchain_path
    );
    run_script(&snapshot_script, working_directory, &[toolchain_path]).await?;

    let snapshot_exists_after: bool = match fs::try_exists(&snapshot_path).await {
        Ok(exists) => exists,
        Err(error) => {
            warn_with_policy!(
                "prepare_l2_artifacts(): failed to check snapshot after generation (path={}, \
                 error={})",
                snapshot_path.display(),
                error
            );
            false
        },
    };
    let initramfs_exists_after: bool = match fs::try_exists(&initramfs_path).await {
        Ok(exists) => exists,
        Err(error) => {
            warn_with_policy!(
                "prepare_l2_artifacts(): failed to check initramfs after generation (path={}, \
                 error={})",
                initramfs_path.display(),
                error
            );
            false
        },
    };

    if snapshot_exists_after && initramfs_exists_after {
        info!(
            "prepare_l2_artifacts(): generated L2 artifacts (snapshot={}, initramfs={})",
            snapshot_path.display(),
            initramfs_path.display()
        );
        Ok(())
    } else {
        let reason: String = format!(
            "L2 artifacts missing after generation attempt (snapshot={}, initramfs={})",
            snapshot_path.display(),
            initramfs_path.display()
        );
        warn_with_policy!("prepare_l2_artifacts(): {reason}");
        Err(::anyhow::anyhow!(reason))
    }
}

///
/// # Description
///
/// Cleans stale artifacts left after a Nanvix Daemon run, mirroring the teardown logic from the
/// reference shell runner. When L2 is enabled, removes Nanvix network namespaces and waits for
/// TCP TIME_WAIT sockets to clear.
///
/// # Parameters
///
/// - `l2_enabled`: Indicates whether the finished run was deployed with L2 networking.
/// - `http_port`: Optional TCP port exposed by the Nanvix Daemon HTTP endpoint.
/// - `tmp_directory`: Directory inspected for stale Nanvix artifacts.
/// - `tcp_cleanup_max_wait_seconds`: Maximum seconds spent waiting for lingering TIME_WAIT
///   sockets.
/// - `tcp_cleanup_poll_interval_seconds`: Seconds between TIME_WAIT socket inspections.
///
pub(crate) async fn cleanup_after_run(
    l2_enabled: bool,
    http_port: Option<u16>,
    tmp_directory: &Path,
    tcp_cleanup_max_wait_seconds: u64,
    tcp_cleanup_poll_interval_seconds: u64,
) {
    cleanup_stale_unix_sockets(tmp_directory).await;
    cleanup_stale_files(tmp_directory).await;
    if l2_enabled {
        cleanup_stale_netns().await;
        match http_port {
            Some(port) => {
                wait_for_tcp_cleanup(
                    port,
                    tcp_cleanup_max_wait_seconds,
                    tcp_cleanup_poll_interval_seconds,
                )
                .await;
            },
            None => warn_with_policy!(
                "cleanup_after_run(): skipping TCP cleanup because HTTP port is unknown"
            ),
        }
    }
}

///
/// # Description
///
/// Executes a helper script within the Nanvix workspace, surfacing failures as `Result` errors.
///
/// # Parameters
///
/// - `script_path`: Absolute path to the script that should be executed.
/// - `working_directory`: Directory set as the script's current working directory.
/// - `args`: Additional arguments forwarded to the script.
///
/// # Return Value
///
/// Returns `Ok(())` when the script exists and exits successfully; otherwise returns an error
/// describing the missing file, spawn failure, or non-zero exit status.
///
/// # Errors
///
/// Returns an error when the script does not exist, cannot be executed, or exits with a
/// non-zero status code.
///
async fn run_script(script_path: &Path, working_directory: &Path, args: &[&str]) -> Result<()> {
    // Check script existence.
    match fs::try_exists(script_path).await {
        Ok(true) => {},
        Ok(false) => {
            let reason: String = format!(
                "script not found (script={}, cwd={})",
                script_path.display(),
                working_directory.display()
            );
            warn_with_policy!("run_script(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        },
        Err(error) => {
            let reason: String = format!(
                "failed to check script existence (script={}, error={error})",
                script_path.display()
            );
            warn_with_policy!("run_script(): {reason}");
            return Err(::anyhow::anyhow!(reason));
        },
    }

    let status: ::std::process::ExitStatus = Command::new(script_path)
        .current_dir(working_directory)
        .args(args)
        .status()
        .await
        .map_err(|error| {
            let reason: String = format!(
                "failed to execute script (script={}, error={error})",
                script_path.display()
            );
            warn_with_policy!("run_script(): {reason}");
            ::anyhow::anyhow!(reason)
        })?;

    if !status.success() {
        let reason: String = format!(
            "script exited unsuccessfully (script={}, status={status})",
            script_path.display()
        );
        warn_with_policy!("run_script(): {reason}");
        return Err(::anyhow::anyhow!(reason));
    }

    Ok(())
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

///
/// # Description
///
/// Removes Nanvix network namespaces and associated host veth pairs left behind by previous runs.
///
async fn cleanup_stale_netns() {
    let output: ::std::process::Output = match Command::new("sudo")
        .arg("ip")
        .arg("netns")
        .arg("list")
        .output()
        .await
    {
        Ok(output) => output,
        Err(error) => {
            warn_with_policy!(
                "cleanup_stale_netns(): failed to list namespaces via sudo (error={})",
                error
            );
            return;
        },
    };

    if !output.status.success() {
        warn_with_policy!("cleanup_stale_netns(): ip netns list returned status {}", output.status);
        return;
    }

    let stdout: String = String::from_utf8_lossy(&output.stdout).to_string();
    let mut namespaces: Vec<String> = stdout
        .split_whitespace()
        .filter(|token| token.starts_with(NETNS_NAME_PREFIX))
        .map(|token| token.trim().to_string())
        .collect();

    namespaces.sort();
    namespaces.dedup();

    if namespaces.is_empty() {
        debug!("cleanup_stale_netns(): no stale namespaces found");
        return;
    }

    info!("cleanup_stale_netns(): removing {} stale namespace(s)", namespaces.len());
    for namespace in namespaces {
        let ns_id: &str = match namespace.strip_prefix(NETNS_NAME_PREFIX) {
            Some(id) => id,
            None => continue,
        };
        let veth_name: String = format!("{VETH_HOST_PREFIX}{ns_id}");

        match Command::new("sudo")
            .arg("ip")
            .arg("link")
            .arg("del")
            .arg(&veth_name)
            .output()
            .await
        {
            Err(error) => warn_with_policy!(
                "cleanup_stale_netns(): failed to delete veth {} (error={})",
                veth_name,
                error
            ),
            Ok(output) => {
                if !output.status.success() {
                    let stderr: String = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    if stderr.contains("Cannot find device") {
                        // Missing host veth should not be a warning because stale namespaces may
                        // already be cleaned.
                        info!("cleanup_stale_netns(): host veth {} already absent", veth_name);
                    } else {
                        warn_with_policy!(
                            "cleanup_stale_netns(): ip link del {} exited with status {} \
                             (stderr={})",
                            veth_name,
                            output.status,
                            stderr
                        );
                    }
                }
            },
        }

        match Command::new("sudo")
            .arg("ip")
            .arg("netns")
            .arg("del")
            .arg(&namespace)
            .status()
            .await
        {
            Err(error) => warn_with_policy!(
                "cleanup_stale_netns(): failed to delete namespace {} (error={})",
                namespace,
                error
            ),
            Ok(status) if !status.success() => warn_with_policy!(
                "cleanup_stale_netns(): ip netns del {} exited with status {}",
                namespace,
                status
            ),
            Ok(_) => {},
        }
    }
}

///
/// # Description
///
/// Waits until lingering TCP TIME_WAIT connections bound to the provided port disappear or the
/// timeout expires.
///
/// # Parameters
///
/// - `port`: TCP port under observation.
/// - `tcp_cleanup_max_wait_seconds`: Maximum seconds spent waiting for TIME_WAIT sockets.
/// - `tcp_cleanup_poll_interval_seconds`: Seconds between TIME_WAIT socket inspections.
///
async fn wait_for_tcp_cleanup(
    port: u16,
    tcp_cleanup_max_wait_seconds: u64,
    tcp_cleanup_poll_interval_seconds: u64,
) {
    let max_wait: Duration = Duration::from_secs(tcp_cleanup_max_wait_seconds);
    let poll_interval: Duration = Duration::from_secs(tcp_cleanup_poll_interval_seconds);
    let start: Instant = Instant::now();

    info!(
        "wait_for_tcp_cleanup(): waiting for TIME_WAIT sockets on port {} (timeout={}s)",
        port, tcp_cleanup_max_wait_seconds
    );

    loop {
        match count_time_wait_connections(port).await {
            Some(0) => {
                info!("wait_for_tcp_cleanup(): TIME_WAIT sockets cleared for port {}", port);
                return;
            },
            Some(count) => {
                debug!(
                    "wait_for_tcp_cleanup(): {} TIME_WAIT sockets still present on port {}",
                    count, port
                );
            },
            None => {
                warn_with_policy!(
                    "wait_for_tcp_cleanup(): unable to inspect TIME_WAIT sockets, skipping wait"
                );
                return;
            },
        }

        if start.elapsed() >= max_wait {
            warn_with_policy!(
                "wait_for_tcp_cleanup(): timeout reached while waiting for TIME_WAIT sockets on \
                 port {}",
                port
            );
            return;
        }

        sleep(poll_interval).await;
    }
}

///
/// # Description
///
/// Counts TIME_WAIT sockets using the `ss` utility when available.
///
/// # Parameters
///
/// - `port`: TCP port under observation.
///
/// # Return Value
///
/// Returns the number of TIME_WAIT sockets bound to the port when `ss` succeeds; otherwise
/// returns `None` when the command fails.
///
async fn count_time_wait_with_ss(port: u16) -> Option<usize> {
    let port_arg: String = port.to_string();
    let output: ::std::process::Output = match Command::new("ss")
        .args(["-tan", "state", "time-wait", "sport", port_arg.as_str()])
        .output()
        .await
    {
        Ok(output) => output,
        Err(error) => {
            debug!("count_time_wait_with_ss(): failed to execute ss (error={})", error);
            return None;
        },
    };

    if !output.status.success() {
        debug!("count_time_wait_with_ss(): ss returned status {}", output.status);
        return None;
    }

    let stdout: String = String::from_utf8_lossy(&output.stdout).to_string();
    let count: usize = stdout
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .count();

    Some(count)
}

///
/// # Description
///
/// Counts TIME_WAIT sockets using the `netstat` utility when `ss` is not available.
///
/// # Parameters
///
/// - `port`: TCP port under observation.
///
/// # Return Value
///
/// Returns the number of TIME_WAIT sockets bound to the port when `netstat` succeeds; otherwise
/// returns `None` when the utility fails.
///
async fn count_time_wait_with_netstat(port: u16) -> Option<usize> {
    let output: ::std::process::Output = match Command::new("netstat").args(["-tan"]).output().await
    {
        Ok(output) => output,
        Err(error) => {
            debug!("count_time_wait_with_netstat(): failed to execute netstat (error={})", error);
            return None;
        },
    };
    if !output.status.success() {
        debug!("count_time_wait_with_netstat(): netstat returned status {}", output.status);
        return None;
    }

    let stdout: String = String::from_utf8_lossy(&output.stdout).to_string();
    let needle: String = format!(":{port}");
    let count: usize = stdout
        .lines()
        .filter(|line| line.contains(needle.as_str()) && line.contains("TIME_WAIT"))
        .count();

    Some(count)
}

///
/// # Description
///
/// Attempts to count TIME_WAIT sockets using the available host tooling.
///
/// # Parameters
///
/// - `port`: TCP port under observation.
///
/// # Return Value
///
/// Returns the TIME_WAIT count reported by either `ss` or `netstat`; returns `None` when both
/// probes fail.
///
async fn count_time_wait_connections(port: u16) -> Option<usize> {
    if let Some(count) = count_time_wait_with_ss(port).await {
        return Some(count);
    }

    count_time_wait_with_netstat(port).await
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

    #[test]
    fn prepare_l2_artifacts_reuses_existing_files() -> Result<()> {
        let rt: ::tokio::runtime::Runtime = ::tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| ::anyhow::anyhow!("failed to build tokio runtime: {e}"))?;
        rt.block_on(async {
            let temp_dir: PathBuf = unique_temp_dir("nvx-artifacts")?;
            let images_dir: PathBuf = temp_dir.join(DEFAULT_L2_SNAPSHOT_DIRECTORY);
            ::tokio::fs::create_dir_all(&images_dir).await?;

            let snapshot_path: PathBuf = images_dir.join(SNAPSHOT_NAME);
            let initramfs_path: PathBuf = images_dir.join(DEFAULT_SNAPSHOT_FILE_NAME);
            ::tokio::fs::write(&snapshot_path, b"snapshot").await?;
            ::tokio::fs::write(&initramfs_path, b"initramfs").await?;

            prepare_l2_artifacts("toolchain", temp_dir.as_path()).await?;

            let snapshot_exists: bool = ::tokio::fs::try_exists(&snapshot_path)
                .await
                .unwrap_or(false);
            let initramfs_exists: bool = ::tokio::fs::try_exists(&initramfs_path)
                .await
                .unwrap_or(false);

            assert!(snapshot_exists, "snapshot must remain when artifacts pre-exist");
            assert!(initramfs_exists, "initramfs must remain when artifacts pre-exist");

            let _ = fs::remove_dir_all(&temp_dir);
            Ok(())
        })
    }

    #[test]
    fn run_script_returns_error_when_missing() -> Result<()> {
        let rt: ::tokio::runtime::Runtime = ::tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| ::anyhow::anyhow!("failed to build tokio runtime: {e}"))?;
        rt.block_on(async {
            let temp_dir: PathBuf = unique_temp_dir("nvx-run-script")?;
            let missing_script: PathBuf = temp_dir.join("missing.sh");

            let result: Result<()> =
                run_script(missing_script.as_path(), temp_dir.as_path(), &[]).await;

            assert!(result.is_err(), "missing script must produce an error");

            let _ = fs::remove_dir_all(&temp_dir);
            Ok(())
        })
    }
}
