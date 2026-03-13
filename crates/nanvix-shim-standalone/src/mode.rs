//! Standalone execution mode implementation.
//!
//! In standalone mode the whole Nanvix kernel runs in a single VM launched by `nanvixd.elf`.
//! The shim:
//! 1. Resolves initrd and ramfs paths from the OCI rootfs
//! 2. Optionally invokes `mkramfs.elf` to build a FAT32 image
//! 3. Spawns `nanvixd.elf` as a child process
//! 4. Manages its lifecycle (start/kill/wait/cleanup)

use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

use nanvix_shim_core::config::NanvixRuntimeConfig;
use nanvix_shim_core::execution::{ExecutionMode, SandboxConfig};
use nanvix_shim_core::state::WorkloadState;

use crate::process::NanvixProcess;

/// Standalone execution mode.
///
/// Runs the full Nanvix kernel inside a single VM:
/// ```text
/// nanvixd.elf [-ramfs /tmp/{id}.img] -- {initrd_binary} [{initrd_args}]
/// ```
pub struct StandaloneMode {
    id: String,
    config: NanvixRuntimeConfig,
    sandbox: Mutex<Option<PreparedSandbox>>,
    process: Mutex<Option<NanvixProcess>>,
    ramfs_image: Mutex<Option<PathBuf>>,
    exit_result: tokio::sync::watch::Sender<Option<u32>>,
    exit_rx: Mutex<tokio::sync::watch::Receiver<Option<u32>>>,
}

/// Internal state after prepare() — everything needed to launch nanvixd.
#[derive(Debug, Clone)]
struct PreparedSandbox {
    initrd_path: PathBuf,
    initrd_args: Vec<String>,
    ramfs_image: Option<PathBuf>,
    stdin: PathBuf,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl StandaloneMode {
    pub fn new(id: String, config: NanvixRuntimeConfig) -> Self {
        let (exit_tx, exit_rx) = tokio::sync::watch::channel(None);
        Self {
            id,
            config,
            sandbox: Mutex::new(None),
            process: Mutex::new(None),
            ramfs_image: Mutex::new(None),
            exit_result: exit_tx,
            exit_rx: Mutex::new(exit_rx),
        }
    }

    /// Build the ramfs FAT32 image from a directory using mkramfs.elf.
    async fn build_ramfs(&self, ramfs_dir: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
        log::info!(
            "[{}] building ramfs: {:?} -> {:?}",
            self.id,
            ramfs_dir,
            output
        );

        let result = tokio::process::Command::new(&self.config.mkramfs_path)
            .arg("-o")
            .arg(output)
            .arg(ramfs_dir)
            .output()
            .await?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            anyhow::bail!(
                "mkramfs failed (exit code {})\nstdout: {}\nstderr: {}",
                result.status.code().unwrap_or(-1),
                stdout.trim(),
                stderr.trim()
            );
        }

        Ok(())
    }
}

#[async_trait]
impl ExecutionMode for StandaloneMode {
    async fn prepare(&self, config: &SandboxConfig) -> anyhow::Result<()> {
        log::info!("[{}] preparing standalone sandbox", self.id);

        // 1. Resolve initrd path against rootfs
        let initrd_path = config.rootfs_path.join(
            config
                .image_config
                .initrd_path
                .strip_prefix('/')
                .unwrap_or(&config.image_config.initrd_path),
        );

        if !initrd_path.exists() {
            anyhow::bail!("initrd binary not found: {:?}", initrd_path);
        }

        // 2. If ramfs_root is set, build the FAT32 image
        let ramfs_image = if let Some(ref ramfs_root) = config.image_config.ramfs_root {
            let ramfs_dir = config.rootfs_path.join(
                ramfs_root
                    .strip_prefix('/')
                    .unwrap_or(ramfs_root),
            );

            if !ramfs_dir.exists() {
                anyhow::bail!("ramfs directory not found: {:?}", ramfs_dir);
            }

            let img_path = config
                .runtime_config
                .temp_dir
                .join(format!("{}.img", config.id));

            self.build_ramfs(&ramfs_dir, &img_path).await?;
            *self.ramfs_image.lock().await = Some(img_path.clone());
            Some(img_path)
        } else {
            None
        };

        // 3. Store prepared state
        *self.sandbox.lock().await = Some(PreparedSandbox {
            initrd_path,
            initrd_args: config.image_config.initrd_args.clone(),
            ramfs_image,
            stdin: config.stdin.clone(),
            stdout: config.stdout.clone(),
            stderr: config.stderr.clone(),
        });

        log::info!("[{}] standalone sandbox prepared", self.id);
        Ok(())
    }

    async fn start(&self) -> anyhow::Result<u32> {
        let sandbox = self
            .sandbox
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("sandbox not prepared — call prepare() first"))?;

        log::info!("[{}] starting standalone workload", self.id);

        // Build command: nanvixd.elf [-ramfs <img>] -- <initrd> [args...]
        let mut cmd = tokio::process::Command::new(&self.config.kernel_path);

        if let Some(ref ramfs) = sandbox.ramfs_image {
            cmd.arg("-ramfs").arg(ramfs);
        }

        for arg in &self.config.extra_args {
            cmd.arg(arg);
        }

        cmd.arg("--");
        cmd.arg(&sandbox.initrd_path);

        for arg in &sandbox.initrd_args {
            cmd.arg(arg);
        }

        // Stdio — for now inherit; a future iteration can wire to named pipes
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);

        log::info!("[{}] nanvixd started with pid={}", self.id, pid);

        // Spawn a background task to wait for the process and publish exit
        let exit_tx = self.exit_result.clone();
        let id = self.id.clone();
        tokio::spawn(async move {
            let exit_code = match child.wait().await {
                Ok(status) => status.code().unwrap_or(1) as u32,
                Err(e) => {
                    log::error!("[{}] failed to wait for nanvixd: {}", id, e);
                    1
                }
            };
            log::info!("[{}] nanvixd exited with code {}", id, exit_code);
            let _ = exit_tx.send(Some(exit_code));
        });

        // Store a placeholder process (just the PID, child moved to background task)
        *self.process.lock().await = Some(NanvixProcess::from_pid(pid));

        Ok(pid)
    }

    async fn kill(&self, signal: u32) -> anyhow::Result<()> {
        log::info!(
            "[{}] killing standalone workload (signal={})",
            self.id,
            signal
        );

        let mut proc_guard = self.process.lock().await;
        if let Some(ref mut proc) = *proc_guard {
            proc.kill(signal).await?;
        } else {
            log::warn!("[{}] no running process to kill", self.id);
        }

        Ok(())
    }

    async fn wait(&self) -> (u32, DateTime<Utc>) {
        log::info!("[{}] waiting for standalone workload", self.id);

        let mut rx = self.exit_rx.lock().await.clone();
        // Wait until the exit_result has a value
        loop {
            if let Some(exit_code) = *rx.borrow() {
                return (exit_code, Utc::now());
            }
            if rx.changed().await.is_err() {
                // Sender dropped without sending — process was cleaned up
                return (1, Utc::now());
            }
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        log::info!("[{}] cleaning up standalone sandbox", self.id);

        // Kill process if still running
        {
            let mut proc_guard = self.process.lock().await;
            if let Some(ref mut proc) = *proc_guard {
                let _ = proc.kill(9).await; // SIGKILL
            }
            *proc_guard = None;
        }

        // Remove ramfs image
        if let Some(path) = self.ramfs_image.lock().await.take() {
            if path.exists() {
                std::fs::remove_file(&path)?;
                log::info!("[{}] removed ramfs image: {:?}", self.id, path);
            }
        }

        *self.sandbox.lock().await = None;

        Ok(())
    }

    async fn state(&self) -> anyhow::Result<WorkloadState> {
        // Check if the process has exited (via the background wait task)
        let exit_code = *self.exit_rx.lock().await.borrow();
        if let Some(code) = exit_code {
            return Ok(WorkloadState::Stopped {
                exit_code: code,
                exited_at: Utc::now(),
            });
        }

        if self.process.lock().await.is_some() {
            let pid = self.process.lock().await.as_ref().map(|p| p.pid()).unwrap_or(0);
            Ok(WorkloadState::Running { pid })
        } else if self.sandbox.lock().await.is_some() {
            Ok(WorkloadState::Created)
        } else {
            Ok(WorkloadState::Stopped {
                exit_code: 0,
                exited_at: Utc::now(),
            })
        }
    }
}
