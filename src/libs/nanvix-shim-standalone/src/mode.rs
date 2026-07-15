// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

//! Standalone workload runtime implementation.
//!
//! In standalone mode, `nanvixd` runs in HTTP mode as a long-lived server.
//! The shim spawns `nanvixd -http-addr <addr>` during `prepare()`, then uses the
//! HTTP API to spawn (`NEW`) and kill (`KILL`) applications during `start()` and `kill()`.
//!
//! ## Architecture
//!
//! ```text
//! containerd ──ttrpc──▶ shim ──HTTP──▶ nanvixd (HTTP mode)
//!                        │                 │
//!                        │   POST NEW      │  spawns VM with initrd + ramfs
//!                        │   POST KILL     │  kills VM
//!                        │                 │
//! ```
//!
//! ## TODO
//!
//! Currently, `nanvixd` is started per-task in `prepare()` because the `-ramfs` flag is
//! passed at daemon startup time. Ideally, `nanvixd` should be started once per sandbox
//! (in the Sandbox API's `CreateSandbox`), and ramfs should be passed per-application via
//! the `NEW` HTTP request. This requires a nanvixd API change to accept ramfs per-NEW
//! request rather than as a global daemon flag.

//==================================================================================================
// Imports
//==================================================================================================

use std::{
    path::PathBuf,
    process::Stdio,
};

use async_trait::async_trait;
use chrono::{
    DateTime,
    Utc,
};
use tokio::sync::Mutex;

use nanvix_shim_core::{
    config::NanvixRuntimeConfig,
    runtime::{
        SandboxConfig,
        WorkloadRuntime,
    },
    state::WorkloadState,
};

use crate::process::NanvixProcess;

//==================================================================================================
// Constants
//==================================================================================================

/// Default HTTP address for the nanvixd server.
#[allow(dead_code)]
const DEFAULT_HTTP_ADDR: &str = "127.0.0.1:0";

/// Timeout for HTTP requests to nanvixd (in seconds).
const HTTP_TIMEOUT_SECS: u64 = 60;

/// Timeout for waiting for nanvixd HTTP server to become ready (in seconds).
const SERVER_READY_TIMEOUT_SECS: u64 = 30;

//==================================================================================================
// Types
//==================================================================================================

/// Standalone workload runtime.
///
/// Runs `nanvixd` in HTTP mode and manages applications via its REST API.
///
/// # TODO
///
/// Currently, `nanvixd` is started per-task in `prepare()` because the `-ramfs` flag
/// must be passed at daemon startup. Ideally, `nanvixd` should be started once per
/// sandbox (via `CreateSandbox`), and ramfs should be attached per-application in the
/// `NEW` request. This requires changes to the nanvixd HTTP API.
pub struct StandaloneRuntime {
    id: String,
    config: NanvixRuntimeConfig,
    sandbox: Mutex<Option<PreparedSandbox>>,
    /// The nanvixd daemon process.
    daemon: Mutex<Option<NanvixProcess>>,
    /// The HTTP address nanvixd is listening on.
    http_addr: Mutex<Option<String>>,
    /// The user_vm_id returned by nanvixd's NEW response.
    user_vm_id: Mutex<Option<u64>>,
    ramfs_image: Mutex<Option<PathBuf>>,
    exit_result: tokio::sync::watch::Sender<Option<u32>>,
    exit_rx: Mutex<tokio::sync::watch::Receiver<Option<u32>>>,
}

/// Internal state after prepare() — everything needed to launch an application.
#[derive(Debug, Clone)]
struct PreparedSandbox {
    initrd_path: PathBuf,
    initrd_args: Vec<String>,
    _ramfs_image: Option<PathBuf>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl StandaloneRuntime {
    /// Create a new standalone workload runtime.
    pub fn new(id: String, config: NanvixRuntimeConfig) -> Self {
        let (exit_tx, exit_rx) = tokio::sync::watch::channel(None);
        Self {
            id,
            config,
            sandbox: Mutex::new(None),
            daemon: Mutex::new(None),
            http_addr: Mutex::new(None),
            user_vm_id: Mutex::new(None),
            ramfs_image: Mutex::new(None),
            exit_result: exit_tx,
            exit_rx: Mutex::new(exit_rx),
        }
    }

    /// Build the ramfs FAT32 image from a directory using mkramfs.elf.
    async fn build_ramfs(&self, ramfs_dir: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
        log::info!("[{}] building ramfs: {:?} -> {:?}", self.id, ramfs_dir, output);

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

    /// Send an HTTP POST request to nanvixd.
    async fn http_post(
        &self,
        addr: &str,
        message_type: &str,
        body: &str,
    ) -> anyhow::Result<String> {
        use tokio::{
            io::{
                AsyncReadExt,
                AsyncWriteExt,
            },
            net::TcpStream,
        };

        let mut stream = tokio::time::timeout(
            std::time::Duration::from_secs(HTTP_TIMEOUT_SECS),
            TcpStream::connect(addr),
        )
        .await
        .map_err(|_| anyhow::anyhow!("timeout connecting to nanvixd at {}", addr))?
        .map_err(|e| anyhow::anyhow!("failed to connect to nanvixd at {}: {}", addr, e))?;

        let request: String = format!(
            "POST / HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nX-NVX-Message-Type: \
             {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            addr,
            message_type,
            body.len(),
            body
        );

        stream.write_all(request.as_bytes()).await?;

        let mut response: Vec<u8> = Vec::new();
        stream.read_to_end(&mut response).await?;

        let response_str: String = String::from_utf8_lossy(&response).to_string();

        // Extract the body (after the blank line separating headers from body).
        let body_start: usize = response_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
        let response_body: &str = &response_str[body_start..];

        // Check for HTTP error status.
        if !response_str.starts_with("HTTP/1.1 200") && !response_str.starts_with("HTTP/1.0 200") {
            let status_line: &str = response_str.lines().next().unwrap_or("unknown");
            anyhow::bail!("nanvixd HTTP error: {} body: {}", status_line, response_body.trim());
        }

        Ok(response_body.to_string())
    }

    /// Wait for nanvixd HTTP server to become ready.
    ///
    /// Races the TCP readiness probe against the daemon exit channel so that
    /// an early daemon crash is reported immediately instead of waiting for the
    /// full timeout.
    async fn wait_for_server(&self, addr: &str) -> anyhow::Result<()> {
        let deadline =
            tokio::time::Instant::now() + std::time::Duration::from_secs(SERVER_READY_TIMEOUT_SECS);

        let mut exit_rx = self.exit_rx.lock().await.clone();

        loop {
            if tokio::time::Instant::now() > deadline {
                anyhow::bail!(
                    "nanvixd HTTP server not ready after {}s at {}",
                    SERVER_READY_TIMEOUT_SECS,
                    addr
                );
            }

            tokio::select! {
                result = tokio::net::TcpStream::connect(addr) => {
                    match result {
                        Ok(_) => {
                            log::info!("[{}] nanvixd HTTP server is ready at {}", self.id, addr);
                            return Ok(());
                        },
                        Err(_) => {
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        },
                    }
                },
                changed = exit_rx.changed() => {
                    if changed.is_ok() {
                        if let Some(code) = *exit_rx.borrow() {
                            anyhow::bail!(
                                "nanvixd daemon exited (code={}) before HTTP server became ready at {}",
                                code,
                                addr
                            );
                        }
                    } else {
                        anyhow::bail!(
                            "nanvixd exit channel closed before HTTP server became ready at {}",
                            addr
                        );
                    }
                },
            }
        }
    }
}

#[async_trait]
impl WorkloadRuntime for StandaloneRuntime {
    async fn prepare(&self, config: &SandboxConfig) -> anyhow::Result<()> {
        log::info!("[{}] preparing standalone sandbox", self.id);

        // 0. Mount rootfs if containerd provided mount instructions.
        if !config.rootfs_mounts.is_empty() {
            std::fs::create_dir_all(&config.rootfs_path)?;
            for (mount_type, source, options) in &config.rootfs_mounts {
                log::info!(
                    "[{}] mounting rootfs: type={} source={} options={:?}",
                    self.id,
                    mount_type,
                    source,
                    options,
                );
                #[cfg(unix)]
                {
                    use nix::mount::MsFlags;
                    nix::mount::mount(
                        Some(source.as_str()),
                        &config.rootfs_path,
                        Some(mount_type.as_str()),
                        MsFlags::empty(),
                        Some(options.join(",").as_str()),
                    )?;
                }
                #[cfg(windows)]
                {
                    anyhow::bail!(
                        "rootfs mounting not supported on Windows — containerd must pre-unpack \
                         layers"
                    );
                }
            }
        }

        // 1. Resolve initrd path against rootfs.
        let initrd_path: PathBuf = config.rootfs_path.join(
            config
                .image_config
                .initrd_path
                .strip_prefix('/')
                .unwrap_or(&config.image_config.initrd_path),
        );

        if !initrd_path.exists() {
            anyhow::bail!("initrd binary not found: {:?}", initrd_path);
        }

        // 2. If ramfs_root is set, build the FAT32 image.
        let ramfs_image: Option<PathBuf> =
            if let Some(ref ramfs_root) = config.image_config.ramfs_root {
                let ramfs_dir: PathBuf = config
                    .rootfs_path
                    .join(ramfs_root.strip_prefix('/').unwrap_or(ramfs_root));

                if !ramfs_dir.exists() {
                    anyhow::bail!("ramfs directory not found: {:?}", ramfs_dir);
                }

                let img_path: PathBuf = config
                    .runtime_config
                    .temp_dir
                    .join(format!("{}.img", config.id));

                self.build_ramfs(&ramfs_dir, &img_path).await?;
                *self.ramfs_image.lock().await = Some(img_path.clone());
                Some(img_path)
            } else {
                None
            };

        // 3. Start nanvixd in HTTP mode.
        //
        // TODO: nanvixd should be started once per sandbox (in CreateSandbox), not per task.
        // Currently, the -ramfs flag is a global daemon argument, so we must start a new
        // nanvixd instance per task to pass the correct ramfs image. When the nanvixd HTTP
        // API supports per-application ramfs (e.g., via the NEW request body), move this
        // to the Sandbox API layer and reuse a single nanvixd instance across tasks.
        let http_addr: String = format!(
            "127.0.0.1:{}",
            portpicker::pick_unused_port()
                .ok_or_else(|| anyhow::anyhow!("no free TCP port available"))?
        );

        let mut cmd = tokio::process::Command::new(&self.config.kernel_path);
        cmd.arg("-http-addr").arg(&http_addr);

        // Derive the binary directory from the kernel_path so nanvixd can locate
        // kernel.elf regardless of the working directory.
        if let Some(bin_dir) = self.config.kernel_path.parent() {
            cmd.arg("-bin-dir").arg(bin_dir);
        }

        if let Some(ref ramfs) = ramfs_image {
            cmd.arg("-ramfs").arg(ramfs);
        }

        // Forward guest console output to the containerd-provided stdout path when set.
        // This overrides any `-console-file` flag from extra_args so the guest console
        // is written to the path containerd expects to read from.
        let stdout_override = !config.stdout.as_os_str().is_empty();
        let mut skip_next = false;
        for arg in &self.config.extra_args {
            if skip_next {
                skip_next = false;
                continue;
            }
            if stdout_override && arg == "-console-file" {
                skip_next = true;
                continue;
            }
            cmd.arg(arg);
        }
        if stdout_override {
            cmd.arg("-console-file").arg(&config.stdout);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn()?;
        let daemon_pid: u32 = child.id().unwrap_or(0);
        log::info!(
            "[{}] nanvixd HTTP server started: pid={} addr={}",
            self.id,
            daemon_pid,
            http_addr
        );

        *self.daemon.lock().await = Some(NanvixProcess::from_pid(daemon_pid));
        *self.http_addr.lock().await = Some(http_addr.clone());

        // Spawn background task to detect daemon crash.
        let exit_tx = self.exit_result.clone();
        let id_clone = self.id.clone();
        tokio::spawn(async move {
            let exit_code: u32 = match child.wait_with_output().await {
                Ok(output) => {
                    let code: u32 = output.status.code().unwrap_or(1) as u32;
                    if code != 0 {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        log::error!(
                            "[{}] nanvixd daemon exited unexpectedly (code={}): {}",
                            id_clone,
                            code,
                            stderr.trim()
                        );
                    }
                    code
                },
                Err(e) => {
                    log::error!("[{}] failed to wait for nanvixd daemon: {}", id_clone, e);
                    1
                },
            };
            let _ = exit_tx.send(Some(exit_code));
        });

        // Wait for the HTTP server to be ready.
        self.wait_for_server(&http_addr).await?;

        // 4. Store prepared state.
        *self.sandbox.lock().await = Some(PreparedSandbox {
            initrd_path,
            initrd_args: config.image_config.initrd_args.clone(),
            _ramfs_image: ramfs_image,
        });

        log::info!("[{}] standalone sandbox prepared", self.id);
        Ok(())
    }

    async fn start(&self) -> anyhow::Result<u32> {
        let sandbox: PreparedSandbox = self
            .sandbox
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("sandbox not prepared — call prepare() first"))?;

        let http_addr: String = self
            .http_addr
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("nanvixd HTTP server not started"))?;

        log::info!("[{}] spawning application via nanvixd HTTP API", self.id);

        // Pack initrd_args into the program_args format expected by nanvixd.
        // Format: "arg1 arg2;ENV1=val1 ENV2=val2" (semicolon separates args from env).
        let program_args: String = sandbox.initrd_args.join(" ");

        let new_body: String = format!(
            r#"{{"tenant_id":"{}","app_name":"{}","program":"{}","program_args":"{}"}}"#,
            self.id,
            self.id,
            sandbox.initrd_path.display(),
            program_args,
        );

        let response: String = self.http_post(&http_addr, "NEW", &new_body).await?;

        // Parse the response to get user_vm_id.
        let parsed: serde_json::Value = serde_json::from_str(&response).map_err(|e| {
            anyhow::anyhow!("failed to parse NEW response: {} body: {}", e, response)
        })?;

        // user_vm_id can be either a plain integer or a nested object {"value": N}.
        let vm_id: u64 = parsed["user_vm_id"]
            .as_u64()
            .or_else(|| parsed["user_vm_id"]["value"].as_u64())
            .ok_or_else(|| anyhow::anyhow!("missing user_vm_id in NEW response: {}", response))?;

        log::info!("[{}] application spawned: user_vm_id={}", self.id, vm_id);

        *self.user_vm_id.lock().await = Some(vm_id);

        // Spawn a background task that waits for the application to exit.
        //
        // The nanvixd HTTP API's KILL request blocks until the application exits and
        // returns its exit code. We send KILL in the background; when the app finishes
        // naturally, KILL returns immediately with the exit code. If kill() is called
        // explicitly first, it will set exit_result and this task becomes a no-op.
        let exit_tx = self.exit_result.clone();
        let id_clone = self.id.clone();
        let addr_clone = http_addr.clone();
        tokio::spawn(async move {
            // Give the app a moment to start before sending KILL.
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let kill_body: String = format!(r#"{{"user_vm_id":{{"value":{}}}}}"#, vm_id);

            // Use a simple TCP request to send KILL (replicating http_post logic).
            use tokio::io::{
                AsyncReadExt,
                AsyncWriteExt,
            };
            let request: String = format!(
                "POST / HTTP/1.1\r\nHost: {}\r\nContent-Type: \
                 application/json\r\nX-NVX-Message-Type: KILL\r\nContent-Length: \
                 {}\r\nConnection: close\r\n\r\n{}",
                addr_clone,
                kill_body.len(),
                kill_body
            );

            match tokio::net::TcpStream::connect(&addr_clone).await {
                Ok(mut stream) => {
                    let _ = stream.write_all(request.as_bytes()).await;
                    let mut response: Vec<u8> = Vec::new();
                    let _ = stream.read_to_end(&mut response).await;
                    let response_str: String = String::from_utf8_lossy(&response).to_string();

                    let body_start: usize =
                        response_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
                    let body: &str = &response_str[body_start..];

                    let exit_code: u32 = serde_json::from_str::<serde_json::Value>(body)
                        .ok()
                        .and_then(|v| v["exit_code"].as_u64())
                        .unwrap_or(0) as u32;

                    log::info!("[{}] application exited (exit_code={})", id_clone, exit_code);
                    let _ = exit_tx.send(Some(exit_code));
                },
                Err(e) => {
                    log::warn!(
                        "[{}] could not send KILL (daemon may have crashed): {}",
                        id_clone,
                        e
                    );
                    // Don't signal exit here — the daemon crash watcher will do it.
                },
            }
        });

        // Return the daemon PID as the task PID.
        let pid: u32 = self
            .daemon
            .lock()
            .await
            .as_ref()
            .map(|p| p.pid())
            .unwrap_or(0);

        Ok(pid)
    }

    async fn kill(&self, signal: u32) -> anyhow::Result<()> {
        log::info!("[{}] killing standalone workload (signal={})", self.id, signal);

        let http_addr = self.http_addr.lock().await.clone();
        let vm_id = *self.user_vm_id.lock().await;

        if let (Some(addr), Some(id)) = (http_addr, vm_id) {
            let kill_body: String = format!(r#"{{"user_vm_id":{{"value":{}}}}}"#, id);

            match self.http_post(&addr, "KILL", &kill_body).await {
                Ok(response) => {
                    log::info!("[{}] KILL response: {}", self.id, response.trim());
                    // Parse exit code from response.
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
                        let exit_code: u32 = parsed["exit_code"].as_u64().unwrap_or(0) as u32;
                        let _ = self.exit_result.send(Some(exit_code));
                    } else {
                        // Response wasn't valid JSON — still signal exit.
                        let _ = self.exit_result.send(Some(0));
                    }
                },
                Err(e) => {
                    log::warn!("[{}] KILL request failed (app may have exited): {}", self.id, e);
                    // Signal exit so wait() doesn't hang.
                    let _ = self.exit_result.send(Some(137));
                },
            }
        } else {
            // Fall back to killing the daemon process directly.
            let mut proc_guard = self.daemon.lock().await;
            if let Some(ref mut proc) = *proc_guard {
                proc.kill(signal).await?;
            }
        }

        Ok(())
    }

    async fn wait(&self) -> (u32, DateTime<Utc>) {
        log::info!("[{}] waiting for standalone workload", self.id);

        let mut rx = self.exit_rx.lock().await.clone();
        loop {
            if let Some(exit_code) = *rx.borrow() {
                return (exit_code, Utc::now());
            }
            if rx.changed().await.is_err() {
                return (1, Utc::now());
            }
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        log::info!("[{}] cleaning up standalone sandbox", self.id);

        // Kill daemon if still running.
        {
            let mut proc_guard = self.daemon.lock().await;
            if let Some(ref mut proc) = *proc_guard {
                let _ = proc.kill(9).await;
            }
            *proc_guard = None;
        }

        *self.http_addr.lock().await = None;
        *self.user_vm_id.lock().await = None;

        // Remove ramfs image.
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
        // Check if the daemon has exited (via the background wait task).
        let exit_code = *self.exit_rx.lock().await.borrow();
        if let Some(code) = exit_code {
            return Ok(WorkloadState::Stopped {
                exit_code: code,
                exited_at: Utc::now(),
            });
        }

        // The application is running only after start() has assigned a user_vm_id.
        // The daemon being alive (after prepare) does not mean the app is running.
        if self.user_vm_id.lock().await.is_some() {
            let pid: u32 = self
                .daemon
                .lock()
                .await
                .as_ref()
                .map(|p| p.pid())
                .unwrap_or(0);
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
