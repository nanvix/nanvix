// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Workload runtime interface and supporting types.

use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{
    DateTime,
    Utc,
};

use crate::config::NanvixRuntimeConfig;
use nanvix_oci::NanvixImageConfig;

/// Configuration for preparing a sandbox, assembled by the protocol layer from the OCI bundle and
/// runtime configuration.
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
    /// On Linux, this is typically an overlayfs mount combining all OCI image layers. The runtime
    /// mounts these to make rootfs contents accessible. Each entry contains
    /// `(mount_type, source, options)`.
    pub rootfs_mounts: Vec<(String, String, Vec<String>)>,
}

/// Runtime interface for standalone Nanvix workloads.
///
/// The protocol layer delegates sandbox preparation and lifecycle operations to this interface.
#[async_trait]
pub trait WorkloadRuntime: Send + Sync + 'static {
    /// Prepare the sandbox from an OCI bundle.
    async fn prepare(&self, config: &SandboxConfig) -> anyhow::Result<()>;

    /// Start the Nanvix workload and return its task identifier.
    async fn start(&self) -> anyhow::Result<u32>;

    /// Send a signal to the running workload.
    async fn kill(&self, signal: u32) -> anyhow::Result<()>;

    /// Wait for the workload to exit and return its exit code and timestamp.
    async fn wait(&self) -> (u32, DateTime<Utc>);

    /// Clean up temporary files, processes, and state directories.
    async fn cleanup(&self) -> anyhow::Result<()>;

    /// Return the current workload state.
    async fn state(&self) -> anyhow::Result<crate::state::WorkloadState>;
}
