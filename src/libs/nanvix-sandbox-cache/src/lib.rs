// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox cache management for Nanvix.
//!
//! This library provides caching functionality for sandboxed execution environments. It maintains
//! a registry of active Linux Daemon and User VM instances, manages their lifecycle, and handles
//! the control-plane socket connections for communication with these instances.

//==================================================================================================
// Exports
//==================================================================================================

pub use ::nanvix_sandbox::{
    syscomm,
    HwLoc,
    SandboxTag,
};
pub use ::nanvix_sandbox_config::SandboxCacheConfig;

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::nanvix_sandbox::{
    control_plane_sockaddr_builder,
    gateway_sockaddr_builder,
    linuxd::{
        LinuxDaemon,
        PendingLinuxDaemon,
    },
    syscomm::{
        SocketListener,
        SocketStream,
        SocketType,
        UnboundSocket,
    },
    user_vm_sockaddr_builder,
    ControlPlaneAcceptor,
    InitializedSandbox,
    LinuxDaemonArgs,
    RunningSandbox,
    SandboxConfig,
    UninitializedSandbox,
    UserVmIdentifier,
    CONTROL_PLANE_ACCEPT_TIMEOUT,
};
use ::std::{
    collections::HashMap,
    sync::Arc,
};
use ::tokio::{
    sync::{
        oneshot::Receiver,
        RwLock,
    },
    time::timeout,
};

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
/// A cache of active sandboxes and their associated resources.
///
/// This structure maintains a registry of all running sandboxes, Linux Daemon instances,
/// and their control plane connections. It handles sandbox creation, lifecycle management,
/// and resource cleanup for the Nanvix Daemon.
///
/// # Type Parameters
///
/// - `T`: Custom state type. Use `()` if no custom state is required.
///
pub struct SandboxCache<T> {
    /// Configuration parameters for all sandboxes.
    config: SandboxCacheConfig<T>,
    /// Registry of all currently running sandboxes indexed by their unique User VM identifier.
    running_sandboxes: RwLock<HashMap<UserVmIdentifier, RunningSandbox>>,
    /// Registry of all tenant's state indexed by the unique tenant ID.
    tenants: RwLock<HashMap<String, Arc<TenantState>>>,
    /// Shared acceptor that routes control-plane connections to waiting children.
    control_plane_acceptor: Arc<ControlPlaneAcceptor>,
}

///
/// # Description
///
/// Per-tenant state used to serialize Linux Daemon creation.
///
struct TenantState {
    /// Optional Linux Daemon handle for this tenant.
    linuxd_instance: RwLock<Option<Arc<LinuxDaemon>>>,
}

///
/// # Description
///
/// Snapshot of the sandbox cache state captured before shutdown.
///
/// This structure records high-level counters that help diagnose why the daemon is
/// still running when the test harness expects it to exit. The data is lightweight
/// enough to log on every shutdown sequence without impacting performance.
///
pub struct SandboxCacheStateSummary {
    running_sandboxes: usize,
    linuxd_instances: usize,
}

impl SandboxCacheStateSummary {
    ///
    /// # Description
    ///
    /// Returns the number of active sandboxes tracked in the cache.
    ///
    pub fn running_sandboxes(&self) -> usize {
        self.running_sandboxes
    }

    ///
    /// # Description
    ///
    /// Returns the number of cached linuxd instances.
    ///
    pub fn linuxd_instances(&self) -> usize {
        self.linuxd_instances
    }
}

impl<T: Sync + Send + Default + 'static> SandboxCache<T> {
    ///
    /// # Description
    ///
    /// Creates a new sandbox cache with interior locking for concurrent access.
    ///
    /// # Parameters
    ///
    /// - `config`: Configuration parameters for the sandbox cache.
    ///
    /// # Returns
    ///
    /// A shared `Arc<Self>` sandbox cache that uses internal `RwLock` and `Mutex` guards for
    /// fine-grained concurrent access.
    ///
    /// # Errors
    ///
    /// This function returns an error if the control plane socket cannot be bound.
    ///
    pub async fn new(config: SandboxCacheConfig<T>) -> Result<Arc<Self>> {
        // Build control plane socket address. The control plane socket address is the same for
        // all sandboxes, so we initialize it once at cache creation time.
        let (control_plane_bind_sockaddr, _connect_addr): (String, String) =
            control_plane_sockaddr_builder(config.tmp_directory())?;

        // Bind control plane socket.
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

        let control_plane_acceptor: Arc<ControlPlaneAcceptor> = ControlPlaneAcceptor::new(
            control_plane_bind_socket,
            control_plane_bind_sockaddr,
            control_plane_bind_socket_type,
        );

        Ok(Arc::new(Self {
            config,
            running_sandboxes: RwLock::new(HashMap::new()),
            tenants: RwLock::new(HashMap::new()),
            control_plane_acceptor,
        }))
    }

    ///
    /// # Description
    ///
    /// Produces a snapshot summarizing the cache state for logging purposes.
    ///
    /// # Returns
    ///
    /// A `SandboxCacheStateSummary` instance describing key counters.
    ///
    pub async fn state_summary(&self) -> SandboxCacheStateSummary {
        let running_sandboxes: usize = self.running_sandboxes.read().await.len();
        let tenants = self.tenants.read().await;
        let mut linuxd_instances: usize = 0;
        for tenant_state in tenants.values() {
            if tenant_state.linuxd_instance.read().await.is_some() {
                linuxd_instances += 1;
            }
        }
        SandboxCacheStateSummary {
            running_sandboxes,
            linuxd_instances,
        }
    }

    ///
    /// # Description
    ///
    /// Gets or creates tenant state.
    ///
    /// This method provides an accessor to a given tenant's state that supports concurrent
    /// requests for the same tenant. It encapsulates the logic of accessing read/write locks and
    /// managing races.
    ///
    /// # Arguments
    ///
    /// - `tenant_id`: tenant identifier.
    ///
    /// # Returns
    ///
    /// The unique tenant state associated to the provided tenant id.
    ///
    async fn get_or_insert_tenant(&self, tenant_id: &str) -> Arc<TenantState> {
        if let Some(state) = self.tenants.read().await.get(tenant_id) {
            return Arc::clone(state);
        }

        let new_state: Arc<TenantState> = Arc::new(TenantState {
            linuxd_instance: RwLock::new(None),
        });

        let mut tenants = self.tenants.write().await;
        // After acquiring a write lock, check if another task already provisioned the tenant
        // state.
        match tenants.get(tenant_id) {
            Some(existing) => Arc::clone(existing),
            None => {
                tenants.insert(tenant_id.to_string(), Arc::clone(&new_state));
                new_state
            },
        }
    }

    ///
    /// # Description
    ///
    /// Gets or creates the Linux Daemon for the target tenant.
    ///
    /// This method provides a safe accessor to the linux daemon, such that concurrent requests
    /// from the same tenant are serialized around linuxd creation, but can otherwise execute in
    /// parallel.
    ///
    /// # Arguments
    ///
    /// - `tenant_state`: reference to the tenant's state.
    /// - `tenant_id`: unique tenant identifier.
    ///
    /// # Returns
    ///
    /// An initialized linuxd daemon.
    ///
    async fn get_or_create_linuxd(
        &self,
        tenant_state: &Arc<TenantState>,
        tenant_id: &str,
    ) -> Result<Arc<LinuxDaemon>> {
        // Fast path: return existing linuxd without serializing.
        if let Some(linuxd) = tenant_state.linuxd_instance.read().await.clone() {
            return Ok(linuxd);
        }

        // Slow path: acquire write lock to serialize creation. Re-check after acquiring the lock
        // in case another task completed initialization while we were waiting.
        let mut linuxd_instance = tenant_state.linuxd_instance.write().await;
        if let Some(linuxd) = linuxd_instance.clone() {
            return Ok(linuxd);
        }

        let (_control_plane_bind_sockaddr, control_plane_connect_sockaddr): (String, String) =
            control_plane_sockaddr_builder(self.config.tmp_directory())?;
        let system_vm_sockaddr: String =
            user_vm_sockaddr_builder(self.config.tmp_directory(), tenant_id)?;

        let linuxd_args: LinuxDaemonArgs<T> = LinuxDaemonArgs::new(
            tenant_id,
            (control_plane_connect_sockaddr, self.config.control_plane_sockaddr_type()),
            (system_vm_sockaddr, self.config.system_vm_sockaddr_type()),
            self.config.hwloc(),
            self.config.linuxd_binary_path().to_string(),
            self.config.log_directory().to_string(),
            self.config.networking_mode().is_enabled(),
        );

        let linuxd: Arc<LinuxDaemon> = {
            // Register interest in the new linuxd instance.
            let control_plane_stream_rx: Receiver<SocketStream> = self
                .control_plane_acceptor
                .register_linuxd(tenant_id)
                .await?;

            // Spawn it.
            let pending_linuxd: PendingLinuxDaemon = match LinuxDaemon::spawn(&linuxd_args).await {
                Ok(linuxd) => linuxd,
                Err(error) => {
                    self.control_plane_acceptor
                        .unregister_linuxd(tenant_id)
                        .await;
                    let reason: String =
                        format!("failed to spawn linuxd (tenant_id={tenant_id}, error={error:?})");
                    error!("get_or_create_linuxd(): {reason}");
                    anyhow::bail!(reason);
                },
            };

            // Await for the linuxd instance to send a handshake message.
            let control_plane_stream: SocketStream =
                match timeout(CONTROL_PLANE_ACCEPT_TIMEOUT, control_plane_stream_rx).await {
                    Ok(Ok(stream)) => stream,
                    Ok(Err(error)) => {
                        self.control_plane_acceptor
                            .unregister_linuxd(tenant_id)
                            .await;
                        pending_linuxd.abort().await;
                        let reason: String = format!(
                            "control-plane acceptor dropped before delivering linuxd stream \
                             (tenant_id={tenant_id}, error={error:?})"
                        );
                        error!("get_or_create_linuxd(): {reason}");
                        anyhow::bail!(reason);
                    },
                    Err(error) => {
                        self.control_plane_acceptor
                            .unregister_linuxd(tenant_id)
                            .await;
                        pending_linuxd.abort().await;
                        let reason: String = format!(
                            "timed-out waiting for linuxd control-plane connection \
                             (tenant_id={tenant_id}, error={error:?})"
                        );
                        error!("get_or_create_linuxd(): {reason}");
                        anyhow::bail!(reason);
                    },
                };

            // Upgrade the pending linuxd instance to a full one by attaching the newly received
            // control-plane stream.
            Arc::new(pending_linuxd.attach_control_plane(control_plane_stream))
        };

        *linuxd_instance = Some(Arc::clone(&linuxd));
        Ok(linuxd)
    }

    ///
    /// # Description
    ///
    /// Gets or creates a sandbox matching the specified parameters.
    ///
    /// If a matching sandbox exists in the cache, returns its information immediately.
    /// Otherwise, creates a new sandbox with the specified configuration, initializes it,
    /// starts it, and adds it to the cache before returning.
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
    /// and the gateway socket type.  On failure, returns an error describing what went wrong.
    ///
    /// # Error Recovery
    ///
    /// If sandbox initialization fails after allocating resources (e.g., Linux Daemon spawns but
    /// User VM fails), resource cleanup follows these guarantees:
    ///
    /// ## Shared Resources (Retained for Reuse)
    ///
    /// - **LinuxDaemon**: Wrapped in `Arc<LinuxDaemon>` and stored per-tenant in the `tenants`
    ///   map (indexed by tenant ID). If Linux Daemon spawns successfully but User VM
    ///   initialization fails, the daemon is **kept** in the cache and reused for subsequent
    ///   sandbox creation attempts within the same tenant. This is intentional: the daemon
    ///   remains operational and can service future requests, avoiding the overhead of
    ///   respawning.
    ///
    /// - **Control Plane Socket**: Wrapped in `Arc<Mutex<(SocketListener, String, SocketType)>>`
    ///   and stored in `control_plane_socket`. Like the Linux Daemon, it is shared across all
    ///   sandboxes for the same tenant. If created but initialization fails, the socket is
    ///   **retained** and reused. The `SocketListener` Drop implementation ensures Unix socket
    ///   files are removed when the last reference is dropped during cache cleanup.
    ///
    /// ## Cache State Guarantees
    ///
    /// - **running_sandboxes**: Only updated after **successful** sandbox startup. Failures during
    ///   initialization or startup do not pollute this map.
    ///
    /// - **tenants**: Tenant state is inserted eagerly on first access (before spawning Linux
    ///   Daemon). The `linuxd_instance` field within the tenant state is only populated after a
    ///   **successful** Linux Daemon spawn, even if User VM initialization fails later. This
    ///   allows daemon reuse across retry attempts.
    ///
    /// ## Arc Reference Counting
    ///
    /// The `LinuxDaemon` and control plane socket are wrapped in `Arc` to enable safe sharing:
    /// - One reference is held in `tenants` or `control_plane_bind_socket`.
    /// - Additional references are held by `InitializedSandbox` and `RunningSandbox` instances.
    /// - When sandboxes are terminated via `kill()` or `cleanup()`, their references are dropped.
    /// - The resources are only destroyed when the last `Arc` reference is dropped (typically
    ///   during cache cleanup).
    ///
    /// ## Retry Safety
    ///
    /// After an initialization or startup error, it is **safe** to retry `get()` with the same
    /// parameters:
    /// - Shared resources (Linux Daemon, control plane socket) are already initialized and will
    ///   be reused.
    /// - No partial state is present in the cache maps that would interfere with retry attempts.
    ///
    pub async fn get(
        &self,
        tenant_id: &str,
        program: &str,
        app_name: &str,
        program_args: Option<String>,
    ) -> Result<(UserVmIdentifier, String, SocketType)> {
        trace!(
            "get(): tenant_id={tenant_id}, program={program}, app_name={app_name}, \
             program_args={program_args:?}"
        );

        // Construct a new tag for sandbox creation.
        let tag: SandboxTag = SandboxTag::new(tenant_id, program, app_name, program_args);

        // Check if a sandbox with this tag already exists in the cache.
        if let Some(sandbox) = self.running_sandboxes.read().await.get(&tag.sandbox_id()) {
            return Ok((
                tag.sandbox_id(),
                sandbox.gateway_socket_info().0.clone(),
                sandbox.gateway_socket_info().1,
            ));
        }
        let tenant_state: Arc<TenantState> = self.get_or_insert_tenant(tag.tenant_id()).await;
        let linuxd: Arc<LinuxDaemon> = self
            .get_or_create_linuxd(&tenant_state, tag.tenant_id())
            .await
            .map_err(|error| {
                let reason: String = format!(
                    "failed to get or create linuxd (tenant_id={}, program={}, app_name={}, \
                     error={error:?})",
                    tag.tenant_id(),
                    tag.program(),
                    tag.app_name()
                );
                error!("get(): {reason}");
                anyhow::anyhow!(reason)
            })?;

        let uninitialized_sandbox: UninitializedSandbox<T> = UninitializedSandbox::new(
            tag.program(),
            tag.program_args().cloned(),
            self.config.ramfs_filename().map(|s| s.to_string()),
            self.control_plane_acceptor.clone(),
        )
        .with_linuxd(linuxd);

        // Work-out socket addresses.
        let (control_plane_bind_sockaddr, control_plane_connect_sockaddr): (String, String) =
            control_plane_sockaddr_builder(self.config.tmp_directory())?;
        let user_vm_sockaddr: String =
            user_vm_sockaddr_builder(self.config.tmp_directory(), tag.tenant_id())?;
        let gateway_sockaddr: String = gateway_sockaddr_builder(
            self.config.tmp_directory(),
            tag.tenant_id(),
            tag.sandbox_id(),
        )?;

        let gateway_socket_address: String = gateway_sockaddr.clone();
        let gateway_socket_type: SocketType = self.config.gateway_sockaddr_type();

        let config: SandboxConfig<T> = SandboxConfig::new(
            tag.tenant_id(),
            tag.sandbox_id(),
            (gateway_socket_address.clone(), gateway_socket_type),
            (user_vm_sockaddr.clone(), self.config.system_vm_sockaddr_type()),
            self.config.console_file().map(|s| s.to_string()),
            self.config.hwloc().clone(),
            self.config.kernel_binary_path(),
            self.config.linuxd_binary_path(),
            self.config.uservm_binary_path(),
            self.config.log_directory(),
            Some((control_plane_bind_sockaddr.clone(), self.config.control_plane_sockaddr_type())),
            (control_plane_connect_sockaddr.clone(), self.config.control_plane_sockaddr_type()),
            self.config.networking_mode().is_enabled(),
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

        // Run sandbox.
        match initialized_sandbox.start(tag.clone()).await {
            Ok(running_sandbox) => {
                self.running_sandboxes
                    .write()
                    .await
                    .insert(tag.sandbox_id(), running_sandbox);
            },
            Err(error) => {
                error!(
                    "get(): failed to start sandbox (tenant_id={}, program={}, app_name={}, \
                     error={error:?})",
                    tag.tenant_id(),
                    tag.program(),
                    tag.app_name()
                );
                return Err(error);
            },
        };

        Ok((tag.sandbox_id(), gateway_sockaddr, gateway_socket_type))
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
    pub async fn kill(&self, user_vm_id: UserVmIdentifier) -> Result<i32> {
        let sandbox: Option<RunningSandbox> =
            self.running_sandboxes.write().await.remove(&user_vm_id);
        if let Some(sandbox) = sandbox {
            match sandbox.shutdown().await {
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
            }
        } else {
            let reason: &str = "user VM instance not found in cache";
            error!("kill(): {reason} (user_vm_id={user_vm_id})");
            Err(anyhow::anyhow!("{reason}"))
        }
    }

    ///
    /// # Description
    ///
    /// Performs cleanup by gracefully shutting down all sandboxes and Linux Daemon instances.
    ///
    /// This method shuts down all User VMs first, then terminates all Linux Daemon instances.
    /// It should be called when the daemon is shutting down to ensure proper resource cleanup.
    ///
    pub async fn cleanup(&self) {
        debug!("cleaning up sandbox cache");

        let running_sandboxes: HashMap<UserVmIdentifier, RunningSandbox> = {
            let mut running_sandboxes = self.running_sandboxes.write().await;
            ::std::mem::take(&mut *running_sandboxes)
        };

        // First shutdown all user VMs.
        for (tag, sandbox) in running_sandboxes {
            debug!("cleaning user vm instance (tag={tag:?})");
            match sandbox.shutdown().await {
                Some(status) => {
                    debug!(
                        "cleanup(): sandbox reported exit status (tag={tag:?}, status={status:?})"
                    );
                },
                None => {
                    warn!(
                        "cleanup(): sandbox shutdown did not complete before timeout (tag={tag:?})"
                    );
                },
            }
        }

        let tenants: HashMap<String, Arc<TenantState>> = {
            let mut tenants = self.tenants.write().await;
            ::std::mem::take(&mut *tenants)
        };

        // Shutdown all linuxd instances.
        for (tenant_id, tenant_state) in tenants {
            let linuxd_instance: Option<Arc<LinuxDaemon>> = {
                let mut linuxd = tenant_state.linuxd_instance.write().await;
                linuxd.take()
            };
            let Some(linuxd_instance) = linuxd_instance else {
                continue;
            };
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

        let summary: SandboxCacheStateSummary = self.state_summary().await;
        debug!(
            "cleanup summary: running_sandboxes={}, linuxd_instances={}",
            summary.running_sandboxes(),
            summary.linuxd_instances(),
        );
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::nanvix_sandbox::syscomm::SocketType;
    use ::nanvix_sandbox_config::NetworkingMode;

    // Constant for test user VM identifier that is guaranteed to not exist.
    const NONEXISTENT_USER_VM_ID: u32 = 99999;

    ///
    /// # Description
    ///
    /// RAII wrapper for temporary test directories that automatically cleans up on drop.
    ///
    struct TempTestDir {
        /// Path to the temporary directory.
        path: String,
    }

    impl TempTestDir {
        ///
        /// # Description
        ///
        /// Creates a new unique temporary directory for testing.
        ///
        /// # Returns
        ///
        /// A `TempTestDir` instance that will clean up the directory when dropped.
        ///
        fn new() -> Self {
            use ::std::sync::atomic::{
                AtomicU64,
                Ordering,
            };

            static COUNTER: AtomicU64 = AtomicU64::new(0);

            let base_tmp: String = ::std::env::temp_dir().to_string_lossy().to_string();
            let unique_id: u64 = ::std::time::SystemTime::now()
                .duration_since(::std::time::UNIX_EPOCH)
                .expect("system time should be after UNIX_EPOCH")
                .as_nanos() as u64;
            let counter: u64 = COUNTER.fetch_add(1, Ordering::Relaxed);
            let unique_dir: String = format!("{}/nanvix-test-{}-{}", base_tmp, unique_id, counter);
            ::std::fs::create_dir_all(&unique_dir).expect("failed to create test directory");
            Self { path: unique_dir }
        }

        ///
        /// # Description
        ///
        /// Returns the path to the temporary directory.
        ///
        fn path(&self) -> &str {
            &self.path
        }
    }

    impl Drop for TempTestDir {
        fn drop(&mut self) {
            // Best-effort cleanup; warn if removal fails.
            if let Err(error) = ::std::fs::remove_dir_all(&self.path) {
                error!("TempTestDir::drop(): failed to remove {} (error={})", self.path, error);
            }
        }
    }

    ///
    /// # Description
    ///
    /// Creates a test configuration.
    ///
    /// # Returns
    ///
    /// A tuple of the sandbox cache configuration and the temp directory handle.
    /// The temp directory is automatically cleaned up when the handle is dropped.
    ///
    fn create_test_config() -> (SandboxCacheConfig<()>, TempTestDir) {
        let tmp_dir: TempTestDir = TempTestDir::new();
        let config: SandboxCacheConfig<()> = SandboxCacheConfig::new(
            SocketType::Unix,
            SocketType::Unix,
            SocketType::Unix,
            None,
            None,
            None,
            &format!("{}/kernel.elf", tmp_dir.path()),
            &format!("{}/linuxd.elf", tmp_dir.path()),
            &format!("{}/uservm.elf", tmp_dir.path()),
            &format!("{}/logs", tmp_dir.path()),
            tmp_dir.path(),
            NetworkingMode::Disabled,
        );
        (config, tmp_dir)
    }

    ///
    /// # Description
    ///
    /// Helper function to create a test configuration with custom parameters.
    ///
    /// # Parameters
    ///
    /// - `console_file`: Optional console file path.
    /// - `hwloc`: Optional hardware locality configuration.
    /// - `socket_type`: Socket type for all connections.
    ///
    /// # Returns
    ///
    /// A tuple of the sandbox cache configuration and the temp directory handle.
    /// The temp directory is automatically cleaned up when the handle is dropped.
    ///
    fn create_custom_test_config(
        console_file: Option<String>,
        hwloc: Option<HwLoc>,
        socket_type: SocketType,
    ) -> (SandboxCacheConfig<()>, TempTestDir) {
        let tmp_dir: TempTestDir = TempTestDir::new();

        let config: SandboxCacheConfig<()> = SandboxCacheConfig::new(
            socket_type,
            socket_type,
            socket_type,
            console_file,
            None,
            hwloc,
            &format!("{}/kernel.elf", tmp_dir.path()),
            &format!("{}/linuxd.elf", tmp_dir.path()),
            &format!("{}/uservm.elf", tmp_dir.path()),
            &format!("{}/logs", tmp_dir.path()),
            tmp_dir.path(),
            NetworkingMode::Disabled,
        );

        (config, tmp_dir)
    }

    ///
    /// # Description
    ///
    /// Tests sandbox cache creation with default configuration.
    ///
    #[tokio::test]
    async fn test_new_creates_cache() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) = create_test_config();
        let result: Result<Arc<SandboxCache<()>>> = SandboxCache::new(config).await;
        assert!(result.is_ok());
    }

    ///
    /// # Description
    ///
    /// Tests sandbox cache creation with multi-process configuration.
    ///
    #[tokio::test]
    async fn test_new_multi_process_mode() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) = create_test_config();
        let result: Result<Arc<SandboxCache<()>>> = SandboxCache::new(config).await;
        assert!(result.is_ok());

        let cache: Arc<SandboxCache<()>> = result.unwrap();
        assert_eq!(cache.running_sandboxes.read().await.len(), 0);
        assert_eq!(cache.tenants.read().await.len(), 0);
    }

    ///
    /// # Description
    ///
    /// Tests that cleanup properly empties all cache structures.
    ///
    #[tokio::test]
    async fn test_cleanup_empties_cache() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) = create_test_config();
        let cache: Arc<SandboxCache<()>> = SandboxCache::new(config).await.unwrap();
        cache.cleanup().await;
        assert_eq!(cache.running_sandboxes.read().await.len(), 0);
    }

    ///
    /// # Description
    ///
    /// Tests that kill returns an error for non-existent sandbox.
    ///
    #[tokio::test]
    async fn test_kill_nonexistent_sandbox_fails() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) = create_test_config();
        let cache: Arc<SandboxCache<()>> = SandboxCache::new(config).await.unwrap();

        let nonexistent_id: UserVmIdentifier = UserVmIdentifier::new(NONEXISTENT_USER_VM_ID);
        let result: Result<i32> = cache.kill(nonexistent_id).await;
        assert!(result.is_err());
    }

    ///
    /// # Description
    ///
    /// Tests that SandboxTag creates unique identifiers.
    ///
    #[test]
    fn test_sandbox_tag_creates_unique_ids() {
        let tag1: SandboxTag =
            SandboxTag::new("tenant1", "/bin/program", "app1", Some("arg1".to_string()));
        let tag2: SandboxTag =
            SandboxTag::new("tenant1", "/bin/program", "app1", Some("arg1".to_string()));

        // Same parameters should create different sandbox IDs.
        assert_ne!(tag1.sandbox_id(), tag2.sandbox_id());
    }

    ///
    /// # Description
    ///
    /// Tests that SandboxTag properly stores and retrieves attributes.
    ///
    #[test]
    fn test_sandbox_tag_attributes() {
        let tenant_id: &str = "tenant1";
        let program: &str = "/bin/program";
        let app_name: &str = "app1";
        let program_args: Option<String> = Some("arg1".to_string());

        let tag: SandboxTag = SandboxTag::new(tenant_id, program, app_name, program_args.clone());

        assert_eq!(tag.tenant_id(), tenant_id);
        assert_eq!(tag.program(), program);
        assert_eq!(tag.program_args(), program_args.as_ref());
    }

    ///
    /// # Description
    ///
    /// Tests that SandboxTag works with no program arguments.
    ///
    #[test]
    fn test_sandbox_tag_no_args() {
        let tag: SandboxTag = SandboxTag::new("tenant1", "/bin/program", "app1", None);
        assert!(tag.program_args().is_none());
    }

    ///
    /// # Description
    ///
    /// Tests SandboxCacheConfig creation and getters.
    ///
    #[test]
    fn test_config_multi_process() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) = create_test_config();
        assert_eq!(config.control_plane_sockaddr_type(), SocketType::Unix);
        assert_eq!(config.gateway_sockaddr_type(), SocketType::Unix);
        assert_eq!(config.system_vm_sockaddr_type(), SocketType::Unix);
        assert!(config.kernel_binary_path().ends_with("/kernel.elf"));
        assert!(config.linuxd_binary_path().ends_with("/linuxd.elf"));
        assert!(config.uservm_binary_path().ends_with("/uservm.elf"));
        assert!(config.log_directory().ends_with("/logs"));
        assert!(config.tmp_directory().contains("nanvix-test"));
    }

    ///
    /// # Description
    ///
    /// Tests SandboxCacheConfig with console file option.
    ///
    #[test]
    fn test_config_with_console_file() {
        let tmp_dir: String = ::std::env::temp_dir().to_string_lossy().to_string();
        let console_file: String = format!("{}/console.log", tmp_dir);
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) =
            create_custom_test_config(Some(console_file.clone()), None, SocketType::Unix);
        assert_eq!(config.console_file(), Some(console_file.as_str()));
    }

    ///
    /// # Description
    ///
    /// Tests SandboxCacheConfig with hwloc option set to None.
    ///
    #[test]
    fn test_config_without_hwloc() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) = create_test_config();
        assert!(config.hwloc().is_none());
    }

    ///
    /// # Description
    ///
    /// Tests SandboxCacheConfig with different socket types.
    ///
    #[test]
    fn test_config_socket_types() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) =
            create_custom_test_config(None, None, SocketType::Tcp);
        assert_eq!(config.control_plane_sockaddr_type(), SocketType::Tcp);
        assert_eq!(config.gateway_sockaddr_type(), SocketType::Tcp);
        assert_eq!(config.system_vm_sockaddr_type(), SocketType::Tcp);
    }
}
