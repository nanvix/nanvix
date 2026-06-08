// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Uninitialized sandbox state management.
//!
//! This module defines the `UninitializedSandbox` structure and its methods for creating
//! and configuring a sandbox before initialization. It provides a builder pattern for
//! setting up Linux Daemon instances, control plane sockets, and sandbox configurations.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use crate::netns::{
    NetnsHandle,
    NetnsInfo,
};
#[cfg(not(feature = "standalone"))]
use crate::ControlPlaneAcceptor;
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use crate::SnapshotDirHandle;
#[cfg(not(feature = "standalone"))]
use crate::{
    config::CONTROL_PLANE_ACCEPT_TIMEOUT,
    linuxd::{
        LinuxDaemon,
        PendingLinuxDaemon,
    },
    LinuxDaemonArgs,
};
use crate::{
    InitializedSandbox,
    SandboxConfig,
};
use ::anyhow::Result;
use ::log::error;
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use ::std::marker::PhantomData;
#[cfg(not(feature = "standalone"))]
use ::std::sync::Arc;
#[cfg(not(feature = "standalone"))]
use ::syscomm::SocketStream;
#[cfg(not(feature = "standalone"))]
use ::tokio::{
    sync::oneshot::Receiver,
    time::timeout,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// An uninitialized sandbox.
///
/// This structure represents a sandbox in its initial state before initialization. It uses
/// a builder pattern to accumulate configuration and resources (Linux Daemon, control plane
/// socket, configuration) before transitioning to an initialized state.
///
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. This is passed to system call handlers in
///   single-process mode. Use `()` if no custom state is required.
///
pub struct UninitializedSandbox<T> {
    /// Path to the guest binary file to execute.
    guest_binary_path: String,
    /// Optional command-line arguments for the program.
    program_args: Option<String>,
    /// Optional RAM filesystem image exposed to the guest program.
    ramfs_filename: Option<String>,
    /// Optional handle to an existing Linux Daemon instance.
    #[cfg(not(feature = "standalone"))]
    linuxd: Option<Arc<LinuxDaemon>>,
    /// Optional handle to a network namespace. Only used in L2 deployments.
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    netns_handle: Option<NetnsHandle>,
    /// Optional handle to the per-instance snapshot directory. Only used in L2 deployments.
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    snapshot_dir_handle: Option<SnapshotDirHandle>,
    /// Shared control-plane acceptor used to route child connections.
    #[cfg(not(feature = "standalone"))]
    control_plane_acceptor: Option<Arc<ControlPlaneAcceptor>>,
    /// Optional sandbox configuration parameters.
    config: Option<SandboxConfig<T>>,
    /// Phantom data to maintain the generic type parameter `T` in the structure.
    /// This is required because `T` is only used in single-process mode for the syscall table.
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    _phantom: PhantomData<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: Sync + Send + Default + 'static> UninitializedSandbox<T> {
    ///
    /// # Description
    ///
    /// Creates a new instance of an uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `guest_binary_path`: Path to the guest binary file to execute.
    /// - `program_args`: Optional command-line arguments for the program.
    /// - `ramfs_filename`: Optional RAM filesystem image filename to expose to the guest.
    /// - `control_plane_acceptor`: Shared control-plane acceptor.
    ///
    /// # Returns
    ///
    /// A new instance of an uninitialized sandbox.
    ///
    pub fn new(
        guest_binary_path: &str,
        program_args: Option<String>,
        ramfs_filename: Option<String>,
        #[cfg(not(feature = "standalone"))] control_plane_acceptor: Arc<ControlPlaneAcceptor>,
    ) -> Self {
        UninitializedSandbox {
            guest_binary_path: guest_binary_path.to_string(),
            program_args,
            ramfs_filename,
            #[cfg(not(feature = "standalone"))]
            linuxd: None,
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            netns_handle: None,
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            snapshot_dir_handle: None,
            #[cfg(not(feature = "standalone"))]
            control_plane_acceptor: Some(control_plane_acceptor),
            config: None,
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            _phantom: PhantomData,
        }
    }

    ///
    /// # Description
    ///
    /// Adds a sandbox configuration to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `config`: Sandbox configuration.
    ///
    /// # Returns
    ///
    /// This function returns the modified uninitialized sandbox.
    ///
    pub fn with_config(mut self, config: SandboxConfig<T>) -> Self {
        self.config = Some(config);
        self
    }

    ///
    /// # Description
    ///
    /// Adds a Linux Daemon instance to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `linuxd`: Shared handle to an existing Linux Daemon instance.
    ///
    /// # Returns
    ///
    /// The modified uninitialized sandbox with the Linux Daemon attached.
    ///
    #[cfg(not(feature = "standalone"))]
    pub fn with_linuxd(mut self, linuxd: Arc<LinuxDaemon>) -> Self {
        self.linuxd = Some(linuxd);
        self
    }

    ///
    /// # Description
    ///
    /// Adds a network namespace handle to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `netns_handle`: Optional handle to a network namespace.
    ///
    /// # Returns
    ///
    /// The modified uninitialized sandbox with the network namespace handle attached.
    ///
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    pub fn with_netns_handle(mut self, netns_handle: Option<NetnsHandle>) -> Self {
        self.netns_handle = netns_handle;
        self
    }

    ///
    /// # Description
    ///
    /// Adds a snapshot directory handle to the target uninitialized sandbox.
    ///
    /// # Parameters
    ///
    /// - `snapshot_dir_handle`: Optional handle to the per-instance snapshot directory.
    ///
    /// # Returns
    ///
    /// The modified uninitialized sandbox with the snapshot directory handle attached.
    ///
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    pub fn with_snapshot_dir_handle(
        mut self,
        snapshot_dir_handle: Option<SnapshotDirHandle>,
    ) -> Self {
        self.snapshot_dir_handle = snapshot_dir_handle;
        self
    }

    ///
    /// # Description
    ///
    /// Returns the network namespace information for this sandbox.
    ///
    /// # Returns
    ///
    /// Returns the network namespace information if available, or `None` otherwise.
    ///
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    pub fn netns_info(&self) -> Option<NetnsInfo> {
        if let Some(netns_handle) = &self.netns_handle {
            netns_handle.netns_info().ok()
        } else {
            None
        }
    }

    ///
    /// # Description
    ///
    /// Initializes the sandbox by spawning the Linux Daemon if not already provided. This
    /// transitions the sandbox from uninitialized to initialized state.
    ///
    /// The control plane socket must be provided via `new()` before calling this method. The
    /// socket is expected to be pre-bound by the caller (typically `SandboxCache::new()`).
    ///
    /// # Returns
    ///
    /// On success, returns an initialized sandbox ready to be started.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sandbox configuration is not provided, if the
    /// control plane socket was not provided, or if the Linux Daemon fails to spawn.
    ///
    pub async fn initialize(mut self) -> Result<InitializedSandbox<T>> {
        // Get sandbox configuration.
        let config: SandboxConfig<T> = match self.config.take() {
            None => {
                let reason: &str = "sandbox configuration not provided";
                error!("initialize(): {reason}");
                anyhow::bail!(reason);
            },
            Some(config) => config,
        };

        #[cfg(not(feature = "standalone"))]
        let control_plane_acceptor: Arc<ControlPlaneAcceptor> =
            match self.control_plane_acceptor.take() {
                Some(control_plane_acceptor) => control_plane_acceptor,
                None => {
                    let reason: &str = "control plane acceptor not provided via new()";
                    error!("initialize(): {reason}");
                    anyhow::bail!(reason);
                },
            };

        // Get Linux Daemon.
        #[cfg(not(feature = "standalone"))]
        let linuxd: Arc<LinuxDaemon> = match self.linuxd.take() {
            // Linux Daemon not yet initialized.
            None => {
                // Build Linux Daemon arguments.
                let linuxd_args: LinuxDaemonArgs<T> = {
                    // Get cloud-hypervisor binary directory.
                    let clh_bin_path: String = match config.clh_bin_path() {
                        None => {
                            let reason: &str = "cloud-hypervisor binary directory not provided \
                                                and linuxd not initialized";
                            error!("initialize(): {reason}");
                            anyhow::bail!(reason);
                        },
                        Some(path) => path.to_string(),
                    };

                    // Get temporary directory.
                    let tmp_directory: String = match config.tmp_directory() {
                        None => {
                            let reason: &str =
                                "temporary directory not provided and linuxd not initialized";
                            error!("initialize(): {reason}");
                            anyhow::bail!(reason);
                        },
                        Some(path) => path.to_string(),
                    };

                    // Get L2 flag.
                    let l2: bool = match config.l2() {
                        None => {
                            let reason: &str = "L2 flag not provided and linuxd not initialized";
                            error!("initialize(): {reason}");
                            anyhow::bail!(reason);
                        },
                        Some(l2) => l2,
                    };

                    LinuxDaemonArgs::new(
                        config.tenant_id(),
                        // We pass linuxd the control plane socket's connect address, which may
                        // depend on the network namespace.
                        (
                            config.control_plane_connect_socket_info().0.clone(),
                            config.control_plane_connect_socket_info().1,
                        ),
                        config.system_vm_socket_info().clone(),
                        config.hwloc(),
                        #[cfg(not(any(feature = "single-process", feature = "standalone")))]
                        config.linuxd_binary_path().to_string(),
                        clh_bin_path,
                        config.log_directory().to_string(),
                        tmp_directory,
                        l2,
                        config.networking_enabled(),
                        #[cfg(feature = "single-process")]
                        config.syscall_table(),
                    )
                };

                // Register interest in control-plane stream for the linuxd instance we are about
                // to spawn.
                let control_plane_stream_rx: Receiver<SocketStream> = control_plane_acceptor
                    .register_linuxd(config.tenant_id())
                    .await?;

                // Spawn linuxd.
                let pending_linuxd: PendingLinuxDaemon = match LinuxDaemon::spawn(
                    &linuxd_args,
                    // Share ownership of netns handle with linux daemon process. The netns is
                    // provisioned upstream, if it is not but we are in L2 mode, spawn will fail.
                    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
                    self.netns_handle.clone(),
                    // Pass ownership of the snapshot dir handle to the linuxd instance.
                    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
                    self.snapshot_dir_handle.take(),
                )
                .await
                {
                    Ok(linuxd) => linuxd,
                    Err(error) => {
                        control_plane_acceptor
                            .unregister_linuxd(config.tenant_id())
                            .await;
                        let reason: String = format!("failed to spawn linuxd (error={error:?})");
                        error!("initialize(): {reason}");
                        anyhow::bail!(reason);
                    },
                };

                // Await the handshake message from the newly spawned linuxd via the control-plane
                // stream.
                let control_plane_stream: SocketStream =
                    match timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, control_plane_stream_rx).await {
                        Ok(Ok(stream)) => stream,
                        Ok(Err(error)) => {
                            control_plane_acceptor
                                .unregister_linuxd(config.tenant_id())
                                .await;
                            pending_linuxd.abort().await;
                            let reason: String = format!(
                                "control-plane acceptor dropped before delivering linuxd stream \
                                 (tenant_id={}, error={error:?})",
                                config.tenant_id()
                            );
                            error!("initialize(): {reason}");
                            anyhow::bail!(reason);
                        },
                        Err(error) => {
                            control_plane_acceptor
                                .unregister_linuxd(config.tenant_id())
                                .await;
                            pending_linuxd.abort().await;
                            let reason: String = format!(
                                "timed-out waiting for linuxd control-plane connection \
                                 (tenant_id={}, error={error:?})",
                                config.tenant_id()
                            );
                            error!("initialize(): {reason}");
                            anyhow::bail!(reason);
                        },
                    };

                // Upgrade the pending linuxd instance to a linuxd one by attaching the newly
                // received control-plane stream.
                Arc::new(pending_linuxd.attach_control_plane(control_plane_stream))
            },
            Some(linuxd) => linuxd,
        };

        Ok(InitializedSandbox {
            guest_binary_path: self.guest_binary_path,
            kernel_binary_path: config.kernel_binary_path().to_string(),
            program_args: self.program_args,
            ramfs_filename: self.ramfs_filename,
            #[cfg(not(feature = "standalone"))]
            linuxd,
            #[cfg(not(feature = "standalone"))]
            control_plane_acceptor,
            sandbox_config: config,
            // Pass ownership of the network namespace to the initialized sandbox.
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            netns_handle: self.netns_handle.take(),
            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
            _phantom: PhantomData,
        })
    }
}
