// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! The `ExecutionMode` trait and supporting types.

use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{
    DateTime,
    Utc,
};

use crate::config::NanvixRuntimeConfig;
use nanvix_oci::NanvixImageConfig;

/// Configuration for preparing a sandbox, assembled by the protocol layer
/// from the OCI bundle and runtime config.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Unique identifier for this sandbox/container.
    pub id: String,
    /// Path to the OCI bundle directory.
    pub bundle_path: PathBuf,
    /// Path to the unpacked rootfs.
    pub rootfs_path: PathBuf,
    /// Parsed Nanvix image configuration from OCI annotations.
    pub image_config: NanvixImageConfig,
    /// Host-level runtime configuration.
    pub runtime_config: NanvixRuntimeConfig,
    /// Path to stdin pipe/file.
    pub stdin: PathBuf,
    /// Path to stdout pipe/file.
    pub stdout: PathBuf,
    /// Path to stderr pipe/file.
    pub stderr: PathBuf,
    /// Rootfs mount instructions from containerd.
    ///
    /// On Linux, this is typically an overlayfs mount combining all OCI image layers.
    /// The execution mode is responsible for mounting these to make rootfs contents
    /// accessible. Each entry contains `(mount_type, source, options)`.
    pub rootfs_mounts: Vec<(String, String, Vec<String>)>,
}

/// Trait for Nanvix execution modes.
///
/// Each execution mode (standalone, distributed, etc.) implements this trait
/// to define how sandboxes are prepared, started, managed, and cleaned up.
///
/// The protocol layer (Task/Sandbox ttrpc services) delegates to this trait,
/// making the execution engine fully pluggable.
#[async_trait]
pub trait ExecutionMode: Send + Sync + 'static {
    /// Prepare the sandbox from an OCI bundle.
    ///
    /// This is called during `Task.Create`. The implementation should:
    /// - Resolve file paths from the rootfs
    /// - Build any filesystem images (e.g., ramfs via mkramfs)
    /// - Prepare the command line for the workload
    async fn prepare(&self, config: &SandboxConfig) -> anyhow::Result<()>;

    /// Start the Nanvix workload. Returns a process/task identifier.
    ///
    /// Called during `Task.Start`.
    async fn start(&self) -> anyhow::Result<u32>;

    /// Send a signal to the running workload.
    ///
    /// Called during `Task.Kill`.
    async fn kill(&self, signal: u32) -> anyhow::Result<()>;

    /// Wait for the workload to exit.
    ///
    /// Returns `(exit_code, exit_timestamp)`. Called during `Task.Wait`.
    async fn wait(&self) -> (u32, DateTime<Utc>);

    /// Clean up all resources (temp files, processes, state directories).
    ///
    /// Called during `Task.Delete`.
    async fn cleanup(&self) -> anyhow::Result<()>;

    /// Return the current state of the workload.
    async fn state(&self) -> anyhow::Result<crate::state::WorkloadState>;
}
