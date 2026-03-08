// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Simplified sandbox cache for single-process deployments.
//!
//! This module provides a lightweight sandbox cache that manages sandbox lifecycle without the
//! multi-process concerns (L2 VMs, network namespaces, external daemon binaries). It is intended
//! for use when the Linux Daemon and User VM are embedded within the same process.

//==================================================================================================
// Public Modules
//==================================================================================================

pub mod config;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::config::SimpleSandboxCacheConfig;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    control_plane_sockaddr_builder,
    gateway_sockaddr_builder,
    linuxd::LinuxDaemon,
    syscomm::{
        SocketListener,
        SocketType,
        UnboundSocket,
    },
    user_vm_sockaddr_builder,
    InitializedSandbox,
    RunningSandbox,
    SandboxConfig,
    SandboxTag,
    UninitializedSandbox,
    UserVmIdentifier,
};
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Arc,
};
use ::tokio::sync::Mutex;

//==================================================================================================
// Constants
//==================================================================================================

/// Default exit code returned when the User VM exit code cannot be retrieved.
pub const DEFAULT_EXIT_CODE: i32 = -1;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A simplified sandbox cache for single-process deployments.
///
/// This cache manages sandbox lifecycle directly using embedded Linux Daemon and User VM
/// instances. It does not handle network namespaces, L2 VMs, or external daemon processes.
/// At most one sandbox runs at a time.
///
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. Use `()` if no custom state is required.
///
pub struct SimpleSandboxCache<T> {
    /// Configuration parameters for all sandboxes.
    config: SimpleSandboxCacheConfig<T>,
    /// The currently running sandbox, if any.
    running_sandbox: Option<(UserVmIdentifier, RunningSandbox)>,
    /// Registry of Linux Daemon instances indexed by tenant ID (one per tenant).
    linuxd_instances: HashMap<String, Arc<LinuxDaemon>>,
    /// Shared control plane listener socket (reused across sandboxes for efficiency).
    control_plane_bind_socket: Option<Arc<Mutex<(SocketListener, String, SocketType)>>>,
}

///
/// # Description
///
/// Snapshot of the simplified sandbox cache state captured for diagnostics.
///
pub struct SimpleSandboxCacheStateSummary {
    has_running_sandbox: bool,
    linuxd_instances: usize,
    has_control_plane_bind_socket: bool,
}

impl SimpleSandboxCacheStateSummary {
    /// Returns whether a sandbox is currently running.
    pub fn has_running_sandbox(&self) -> bool {
        self.has_running_sandbox
    }

    /// Returns the number of cached linuxd instances.
    pub fn linuxd_instances(&self) -> usize {
        self.linuxd_instances
    }

    /// Returns `true` when a control-plane socket listener is cached.
    pub fn has_control_plane_bind_socket(&self) -> bool {
        self.has_control_plane_bind_socket
    }
}

impl<T: Sync + Send + Default + 'static> SimpleSandboxCache<T> {
    ///
    /// # Description
    ///
    /// Creates a new simplified sandbox cache wrapped in a shared mutex.
    ///
    /// # Parameters
    ///
    /// - `config`: Configuration parameters for the sandbox cache.
    ///
    /// # Returns
    ///
    /// A shared, mutex-protected sandbox cache ready for concurrent access.
    ///
    pub async fn new(config: SimpleSandboxCacheConfig<T>) -> Result<Arc<Mutex<Self>>> {
        let control_plane_bind_sockaddr: String = {
            let (bind_addr, _connect_addr): (String, String) =
                control_plane_sockaddr_builder(config.tmp_directory())?;
            bind_addr
        };

        let control_plane_bind_socket_type: SocketType = config.control_plane_sockaddr_type();
        let unbound_socket: UnboundSocket = UnboundSocket::new(control_plane_bind_socket_type);
        let control_plane_bind_socket: SocketListener =
            match unbound_socket.bind(&control_plane_bind_sockaddr).await {
                Ok(listener) => listener,
                Err(error) => {
                    let reason: String = format!(
                        "failed to bind control-plane socket \
                         (control_plane_bind_socket_address={control_plane_bind_sockaddr}, \
                         error={error:?})"
                    );
                    error!("new(): {reason}");
                    anyhow::bail!(reason);
                },
            };

        Ok(Arc::new(Mutex::new(Self {
            config,
            running_sandbox: None,
            linuxd_instances: HashMap::new(),
            control_plane_bind_socket: Some(Arc::new(Mutex::new((
                control_plane_bind_socket,
                control_plane_bind_sockaddr,
                control_plane_bind_socket_type,
            )))),
        })))
    }

    ///
    /// # Description
    ///
    /// Produces a snapshot summarizing the cache state for logging purposes.
    ///
    pub fn state_summary(&self) -> SimpleSandboxCacheStateSummary {
        SimpleSandboxCacheStateSummary {
            has_running_sandbox: self.running_sandbox.is_some(),
            linuxd_instances: self.linuxd_instances.len(),
            has_control_plane_bind_socket: self.control_plane_bind_socket.is_some(),
        }
    }

    ///
    /// # Description
    ///
    /// Gets or creates a sandbox matching the specified parameters.
    ///
    /// # Parameters
    ///
    /// - `tenant_id`: Tenant identifier for resource isolation.
    /// - `program`: Path to the program binary to execute.
    /// - `app_name`: Application name for identification.
    /// - `program_args`: Optional command-line arguments for the program.
    ///
    /// # Returns
    ///
    /// On success, returns a tuple containing the User VM identifier, the gateway socket address
    /// and the gateway socket type. On failure, returns an error describing what went wrong.
    ///
    pub async fn get(
        &mut self,
        tenant_id: &str,
        program: &str,
        app_name: &str,
        program_args: Option<String>,
    ) -> Result<(UserVmIdentifier, String, SocketType)> {
        trace!(
            "get(): tenant_id={tenant_id}, program={program}, app_name={app_name}, \
             program_args={program_args:?}"
        );

        let tag: SandboxTag = SandboxTag::new(tenant_id, program, app_name, program_args);

        // Check if sandbox is in cache.
        match &self.running_sandbox {
            Some((id, sandbox)) if *id == tag.sandbox_id() => Ok((
                tag.sandbox_id(),
                sandbox.gateway_socket_info().0.clone(),
                sandbox.gateway_socket_info().1,
            )),
            _ => {
                let control_plane_bind_socket: Arc<Mutex<(SocketListener, String, SocketType)>> =
                    self.control_plane_bind_socket.clone().ok_or_else(|| {
                        let reason: &str = "control plane socket not initialized";
                        error!("get(): {reason}");
                        anyhow::anyhow!(reason)
                    })?;

                let uninitialized_sandbox: UninitializedSandbox<T> = UninitializedSandbox::new(
                    tag.program(),
                    tag.program_args().cloned(),
                    self.config.ramfs_filename().map(|s| s.to_string()),
                    control_plane_bind_socket,
                );

                let gateway_l2_port: Option<crate::tcp_port::TcpPort> = None;

                // Attach existing Linux Daemon if one exists for this tenant.
                let uninitialized_sandbox: UninitializedSandbox<T> =
                    if let Some(linuxd) = self.linuxd_instances.get(tag.tenant_id()) {
                        uninitialized_sandbox.with_linuxd(linuxd.clone())
                    } else {
                        uninitialized_sandbox
                    };

                // Build socket addresses.
                let (control_plane_bind_sockaddr, control_plane_connect_sockaddr): (
                    String,
                    String,
                ) = control_plane_sockaddr_builder(self.config.tmp_directory())?;

                let user_vm_sockaddr: String =
                    user_vm_sockaddr_builder(self.config.tmp_directory(), tag.tenant_id())?;

                let gateway_sockaddr: String = gateway_sockaddr_builder(
                    self.config.tmp_directory(),
                    tag.tenant_id(),
                    tag.sandbox_id(),
                )?;

                let gateway_socket_address: String = gateway_sockaddr.clone();
                let gateway_socket_type: SocketType = self.config.gateway_sockaddr_type();

                // Create per-tenant temporary directory.
                let sandbox_tmp_dir: PathBuf =
                    PathBuf::from(self.config.tmp_directory()).join(tag.tenant_id());
                if let Err(error) = fs::create_dir_all(&sandbox_tmp_dir) {
                    let reason: String = format!(
                        "failed to create sandbox temporary directory (tenant_id={}, program={}, \
                         app_name={}, tmp_dir={sandbox_tmp_dir:?}, error={error:?})",
                        tag.tenant_id(),
                        tag.program(),
                        tag.app_name()
                    );
                    error!("get(): {reason}");
                    anyhow::bail!(reason);
                }

                #[cfg(feature = "single-process")]
                let syscall_table = self.config.syscall_table();

                let toolchain_binary_directory =
                    Some(self.config.toolchain_binary_directory().to_string());

                let config: SandboxConfig<T> = SandboxConfig::new(
                    tag.tenant_id(),
                    tag.sandbox_id(),
                    (gateway_socket_address.clone(), gateway_socket_type, gateway_l2_port),
                    (user_vm_sockaddr.clone(), self.config.system_vm_sockaddr_type()),
                    self.config.console_file().map(|s| s.to_string()),
                    self.config.hwloc().clone(),
                    self.config.kernel_binary_path(),
                    self.config.log_directory(),
                    #[cfg(feature = "single-process")]
                    syscall_table,
                    Some((
                        control_plane_bind_sockaddr.clone(),
                        self.config.control_plane_sockaddr_type(),
                    )),
                    (
                        control_plane_connect_sockaddr.clone(),
                        self.config.control_plane_sockaddr_type(),
                    ),
                    toolchain_binary_directory,
                    Some(sandbox_tmp_dir.to_string_lossy().into_owned()),
                    Some(false), // l2 (not supported in single-process/standalone)
                );

                let uninitialized_sandbox: UninitializedSandbox<T> =
                    uninitialized_sandbox.with_config(config);

                let initialized_sandbox: InitializedSandbox<T> =
                    match uninitialized_sandbox.initialize().await {
                        Ok(sandbox) => sandbox,
                        Err(error) => {
                            error!(
                                "get(): failed to initialize sandbox (tenant_id={}, program={}, \
                                 app_name={}, error={error:?})",
                                tag.tenant_id(),
                                tag.program(),
                                tag.app_name()
                            );
                            return Err(error);
                        },
                    };

                // Cache Linux Daemon instance.
                self.linuxd_instances
                    .insert(tag.tenant_id().to_string(), initialized_sandbox.linuxd());

                // Start sandbox.
                match initialized_sandbox.start(tag.clone()).await {
                    Ok(running_sandbox) => {
                        self.running_sandbox = Some((tag.sandbox_id(), running_sandbox));
                    },
                    Err(error) => {
                        error!(
                            "get(): failed to start sandbox (tenant_id={}, program={}, \
                             app_name={}, error={error:?})",
                            tag.tenant_id(),
                            tag.program(),
                            tag.app_name()
                        );
                        return Err(error);
                    },
                };

                Ok((tag.sandbox_id(), gateway_sockaddr, gateway_socket_type))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Terminates and removes a sandbox from the cache by User VM identifier.
    ///
    /// # Parameters
    ///
    /// - `user_vm_id`: Identifier of the User VM to terminate.
    ///
    /// # Returns
    ///
    /// On success, returns the exit code of the User VM. On failure, returns an error if the
    /// User VM identifier was not found in the cache or if the shutdown did not complete.
    ///
    pub async fn kill(&mut self, user_vm_id: UserVmIdentifier) -> Result<i32> {
        match self.running_sandbox.take() {
            Some((id, sandbox)) if id == user_vm_id => match sandbox.shutdown().await {
                Some(status) => {
                    let exit_code: i32 = status.code().unwrap_or(DEFAULT_EXIT_CODE);
                    if status.success() {
                        debug!(
                            "kill(): sandbox exited successfully (user_vm_id={user_vm_id}, \
                             exit_code={exit_code})"
                        );
                    } else {
                        debug!(
                            "kill(): sandbox exited with non-zero exit code \
                             (user_vm_id={user_vm_id}, exit_code={exit_code})"
                        );
                    }
                    Ok(exit_code)
                },
                None => {
                    warn!(
                        "kill(): sandbox shutdown did not complete before timeout \
                         (user_vm_id={user_vm_id})"
                    );
                    Ok(DEFAULT_EXIT_CODE)
                },
            },
            Some(entry) => {
                // Put back — it wasn't the one requested.
                self.running_sandbox = Some(entry);
                let reason: &str = "user VM instance not found in cache";
                error!("kill(): {reason} (user_vm_id={user_vm_id})");
                Err(anyhow::anyhow!("{reason}"))
            },
            None => {
                let reason: &str = "user VM instance not found in cache";
                error!("kill(): {reason} (user_vm_id={user_vm_id})");
                Err(anyhow::anyhow!("{reason}"))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Performs cleanup by gracefully shutting down all sandboxes and Linux Daemon instances.
    ///
    pub async fn cleanup(&mut self) {
        debug!("cleaning up sandbox cache");

        // Shutdown the running sandbox if present.
        if let Some((id, sandbox)) = self.running_sandbox.take() {
            debug!("cleaning user vm instance (id={id:?})");
            match sandbox.shutdown().await {
                Some(status) => {
                    debug!(
                        "cleanup(): sandbox reported exit status (id={id:?}, status={status:?})"
                    );
                },
                None => {
                    warn!(
                        "cleanup(): sandbox shutdown did not complete before timeout (id={id:?})"
                    );
                },
            }
        }

        // Shutdown all linuxd instances.
        for (tenant_id, linuxd_instance) in self.linuxd_instances.drain() {
            debug!("cleanup(): cleaning linuxd instance (tenant_id={tenant_id:?})");
            let strong_count: usize = Arc::strong_count(&linuxd_instance);
            if strong_count > 1 {
                warn!(
                    "cleanup(): linuxd has {} outstanding Arc references (tenant_id={tenant_id})",
                    strong_count - 1
                );
            }
            linuxd_instance.shutdown().await;
        }

        let summary: SimpleSandboxCacheStateSummary = self.state_summary();
        debug!(
            "cleanup summary: has_running_sandbox={}, linuxd_instances={}, control_plane_socket={}",
            summary.has_running_sandbox(),
            summary.linuxd_instances(),
            summary.has_control_plane_bind_socket(),
        );
    }
}
