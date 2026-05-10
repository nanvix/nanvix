// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Integration tests for the Nanvix containerd shim.
//!
//! These tests exercise the full `StandaloneMode` lifecycle using real Nanvix binaries.
//! They require Nanvix binaries. Set `NANVIX_DIR` to the Nanvix source tree root
//! (e.g., `export NANVIX_DIR=$HOME/nanvix`). Binaries are expected at `$NANVIX_DIR/bin/`.
//!
//! Run with:
//!   NANVIX_DIR=/path/to/nanvix cargo test -p containerd-shim-nanvix-v1 --test integration -- --nocapture
//!
//! Tests are skipped if the Nanvix binaries are not found.

use std::{
    collections::HashMap,
    path::PathBuf,
};

use nanvix_oci::NanvixImageConfig;
use nanvix_shim_core::{
    config::NanvixRuntimeConfig,
    execution::{
        ExecutionMode,
        SandboxConfig,
    },
    state::WorkloadState,
};
use nanvix_shim_standalone::StandaloneMode;

/// Resolve paths to the Nanvix binaries, or return None if not found.
struct NanvixPaths {
    nanvixd: PathBuf,
    mkramfs: PathBuf,
    hello_elf: PathBuf,
}

impl NanvixPaths {
    fn discover() -> Option<Self> {
        // TODO(windows): The `HOME` env var is Unix-only. On Windows the equivalent is
        // `USERPROFILE`. When Windows support is fully integrated, use
        // `std::env::var("HOME").or(std::env::var("USERPROFILE"))` or `dirs::home_dir()`
        // to resolve the home directory portably.

        // Prefer NANVIX_DIR env var; fall back to $HOME/nanvix
        let base: PathBuf = if let Ok(dir) = std::env::var("NANVIX_DIR") {
            PathBuf::from(dir).join("bin")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join("nanvix/bin")
        } else {
            return None;
        };

        // TODO(windows): The `.elf` extension is used for Nanvix guest binaries.
        // On Windows the build system may produce different extensions (e.g., `.exe`).
        // When Windows support is fully integrated, determine the correct binary names
        // per platform or make them configurable.
        let paths = Self {
            nanvixd: base.join("nanvixd.elf"),
            mkramfs: base.join("mkramfs.elf"),
            hello_elf: base.join("hello-rust-nostd.elf"),
        };

        if paths.nanvixd.exists() && paths.mkramfs.exists() && paths.hello_elf.exists() {
            Some(paths)
        } else {
            None
        }
    }

    fn runtime_config(&self, temp_dir: &std::path::Path) -> NanvixRuntimeConfig {
        NanvixRuntimeConfig {
            kernel_path: self.nanvixd.clone(),
            mkramfs_path: self.mkramfs.clone(),
            temp_dir: temp_dir.to_path_buf(),
            execution_mode: "standalone".to_string(),
            // TODO(windows): `/dev/null` is Unix-only. On Windows the equivalent is
            // `NUL`. When Windows support is fully integrated, use a `cfg`-gated
            // constant: `if cfg!(windows) { "NUL" } else { "/dev/null" }`.
            extra_args: vec!["-console-file".to_string(), "/dev/null".to_string()],
        }
    }
}

/// Build a fake OCI rootfs with an initrd binary and optional ramfs directory.
fn build_rootfs(rootfs: &std::path::Path, initrd_src: &std::path::Path, with_ramfs: bool) {
    let initrd_dir = rootfs.join("initrd");
    std::fs::create_dir_all(&initrd_dir).unwrap();
    std::fs::copy(initrd_src, initrd_dir.join("app.elf")).unwrap();

    if with_ramfs {
        let ramfs_dir = rootfs.join("ramfs");
        std::fs::create_dir_all(&ramfs_dir).unwrap();
        std::fs::write(ramfs_dir.join("hello.txt"), "Hello from ramfs!").unwrap();
    }
}

fn make_sandbox_config(
    id: &str,
    rootfs: &std::path::Path,
    runtime_config: &NanvixRuntimeConfig,
    with_ramfs: bool,
) -> SandboxConfig {
    SandboxConfig {
        id: id.to_string(),
        bundle_path: rootfs.parent().unwrap().to_path_buf(),
        rootfs_path: rootfs.to_path_buf(),
        image_config: NanvixImageConfig {
            initrd_path: "/initrd/app.elf".to_string(),
            initrd_args: vec![],
            initrd_env: vec![],
            ramfs_root: if with_ramfs {
                Some("/ramfs".to_string())
            } else {
                None
            },
            arch: "x86".to_string(),
            version: None,
            execution_mode: None,
        },
        runtime_config: runtime_config.clone(),
        stdin: PathBuf::new(),
        stdout: PathBuf::new(),
        stderr: PathBuf::new(),
        rootfs_mounts: vec![],
    }
}

// ─── Full lifecycle: no ramfs ────────────────────────────────────────────────

#[tokio::test]
async fn test_standalone_lifecycle_no_ramfs() {
    let paths = match NanvixPaths::discover() {
        Some(p) => p,
        None => {
            eprintln!(
                "SKIP: Nanvix binaries not found. Set NANVIX_DIR to your Nanvix source tree."
            );
            return;
        },
    };

    let tmp = tempfile::tempdir().unwrap();
    let rootfs = tmp.path().join("rootfs");
    build_rootfs(&rootfs, &paths.hello_elf, false);

    let runtime_config = paths.runtime_config(tmp.path());
    let mode = StandaloneMode::new("test-no-ramfs".to_string(), runtime_config.clone());
    let mut sandbox_config = make_sandbox_config("test-no-ramfs", &rootfs, &runtime_config, false);

    // Point stdout to a temp file so we can verify it after execution.
    let stdout_file = tmp.path().join("stdout.log");
    sandbox_config.stdout = stdout_file.clone();

    // Initial state
    assert!(matches!(mode.state().await.unwrap(), WorkloadState::Stopped { .. }));

    // Prepare
    mode.prepare(&sandbox_config).await.unwrap();
    assert!(matches!(mode.state().await.unwrap(), WorkloadState::Created));

    // Start — spawns the application inside nanvixd via the HTTP API.
    let pid = mode.start().await.unwrap();
    assert!(pid > 0, "Expected a valid PID, got {}", pid);
    assert!(matches!(mode.state().await.unwrap(), WorkloadState::Running { .. }));

    // Give the application a moment to execute.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // TODO(windows): Signal 9 (`SIGKILL`) is a Unix concept. On Windows, process
    // termination uses `TerminateProcess` and there is no direct signal equivalent.
    // When Windows support is fully integrated, verify that `StandaloneMode::kill()`
    // handles signal translation internally, or use a platform-appropriate constant here.

    // Kill — terminates the application via the HTTP API and retrieves its exit code.
    mode.kill(9).await.unwrap();

    // Wait — returns the exit code set by kill().
    let (exit_code, _) = mode.wait().await;
    eprintln!("  nanvixd exited with code {}", exit_code);

    // Verify stdout — the guest console output should have been written to the file.
    let stdout_output = std::fs::read_to_string(&stdout_file)
        .unwrap_or_else(|e| panic!("failed to read stdout file {:?}: {}", stdout_file, e));
    assert!(
        !stdout_output.is_empty(),
        "Expected non-empty stdout from the guest application, but the stdout file was empty"
    );
    eprintln!("  stdout ({} bytes): {}", stdout_output.len(), stdout_output.trim());

    // Cleanup — stops the daemon and removes temp files.
    mode.cleanup().await.unwrap();
    assert!(matches!(mode.state().await.unwrap(), WorkloadState::Stopped { .. }));
}

// ─── Full lifecycle: with ramfs ──────────────────────────────────────────────

#[tokio::test]
async fn test_standalone_lifecycle_with_ramfs() {
    let paths = match NanvixPaths::discover() {
        Some(p) => p,
        None => {
            eprintln!(
                "SKIP: Nanvix binaries not found. Set NANVIX_DIR to your Nanvix source tree."
            );
            return;
        },
    };

    let tmp = tempfile::tempdir().unwrap();
    let rootfs = tmp.path().join("rootfs");
    build_rootfs(&rootfs, &paths.hello_elf, true);

    let runtime_config = paths.runtime_config(tmp.path());
    let mode = StandaloneMode::new("test-with-ramfs".to_string(), runtime_config.clone());
    let mut sandbox_config = make_sandbox_config("test-with-ramfs", &rootfs, &runtime_config, true);

    // Point stdout to a temp file so we can verify it after execution.
    let stdout_file = tmp.path().join("stdout.log");
    sandbox_config.stdout = stdout_file.clone();

    // Prepare — should invoke mkramfs
    mode.prepare(&sandbox_config).await.unwrap();

    // Verify ramfs image was created
    let img_path = tmp.path().join("test-with-ramfs.img");
    assert!(img_path.exists(), "ramfs image not created at {:?}", img_path);
    assert!(std::fs::metadata(&img_path).unwrap().len() > 0, "ramfs image is empty");

    // Start
    let pid = mode.start().await.unwrap();
    assert!(pid > 0);

    // Give the application a moment to execute.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // TODO(windows): Same signal concern as `test_standalone_lifecycle_no_ramfs`.
    // Kill + Wait
    mode.kill(9).await.unwrap();
    let (exit_code, _) = mode.wait().await;
    eprintln!("  nanvixd (with ramfs) exited with code {}", exit_code);

    // Verify stdout — the guest console output should have been written to the file.
    let stdout_output = std::fs::read_to_string(&stdout_file)
        .unwrap_or_else(|e| panic!("failed to read stdout file {:?}: {}", stdout_file, e));
    assert!(
        !stdout_output.is_empty(),
        "Expected non-empty stdout from the guest application, but the stdout file was empty"
    );
    eprintln!("  stdout ({} bytes): {}", stdout_output.len(), stdout_output.trim());

    // Cleanup — should remove the ramfs image
    mode.cleanup().await.unwrap();
    assert!(!img_path.exists(), "ramfs image should have been cleaned up");
}

// ─── Error: missing initrd ──────────────────────────────────────────────────

#[tokio::test]
async fn test_standalone_prepare_missing_initrd() {
    let paths = match NanvixPaths::discover() {
        Some(p) => p,
        None => {
            eprintln!("SKIP: Nanvix binaries not found. Set NANVIX_DIR.");
            return;
        },
    };

    let tmp = tempfile::tempdir().unwrap();
    let rootfs = tmp.path().join("rootfs");
    std::fs::create_dir_all(&rootfs).unwrap();

    let runtime_config = paths.runtime_config(tmp.path());
    let mode = StandaloneMode::new("test-missing".to_string(), runtime_config.clone());
    let sandbox_config = make_sandbox_config("test-missing", &rootfs, &runtime_config, false);

    let result = mode.prepare(&sandbox_config).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("initrd binary not found"),
        "Expected 'initrd binary not found' error"
    );
}

// ─── Error: start without prepare ───────────────────────────────────────────

#[tokio::test]
async fn test_standalone_start_without_prepare() {
    let paths = match NanvixPaths::discover() {
        Some(p) => p,
        None => {
            eprintln!("SKIP: Nanvix binaries not found. Set NANVIX_DIR.");
            return;
        },
    };

    let tmp = tempfile::tempdir().unwrap();
    let runtime_config = paths.runtime_config(tmp.path());
    let mode = StandaloneMode::new("test-no-prepare".to_string(), runtime_config);

    let result = mode.start().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not prepared"));
}

// ─── OCI annotation parsing roundtrip ───────────────────────────────────────

#[tokio::test]
async fn test_oci_annotation_parsing_roundtrip() {
    let mut labels = HashMap::new();
    labels.insert("com.nanvix.os".to_string(), "nanvix".to_string());
    labels.insert("com.nanvix.arch".to_string(), "x86".to_string());
    labels.insert("com.nanvix.initrd.path".to_string(), "/initrd/app.elf".to_string());
    labels.insert("com.nanvix.ramfs.root".to_string(), "/ramfs".to_string());
    labels.insert("com.nanvix.initrd.args".to_string(), "arg1 arg2".to_string());

    let config = NanvixImageConfig::from_annotations(&labels).unwrap();
    assert_eq!(config.initrd_path, "/initrd/app.elf");
    assert_eq!(config.initrd_args, vec!["arg1", "arg2"]);
    assert_eq!(config.ramfs_root, Some("/ramfs".to_string()));
    assert_eq!(config.arch, "x86");
}

// ─── Mode registry ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_mode_registry() {
    use nanvix_shim_core::registry::ModeRegistry;
    use std::sync::Arc;

    let mut registry = ModeRegistry::new();
    registry.register("standalone", |id, config| {
        Arc::new(StandaloneMode::new(id.to_string(), config.clone()))
    });

    let config = NanvixRuntimeConfig::default();

    assert!(registry.create("standalone", "test-id", &config).is_ok());
    assert!(registry.create("unknown", "test-id", &config).is_err());
}
