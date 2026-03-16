// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Sandbox cache management for Nanvix.
//!
//! This library provides caching functionality for sandboxed execution environments. It maintains
//! a registry of active Linux Daemon and User VM instances, manages their lifecycle, and handles
//! the control-plane socket connections for communication with these instances.

//==================================================================================================
// Public Modules
//==================================================================================================

pub mod config;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::config::SandboxCacheConfig;
pub use ::nanvix_sandbox::{
    syscomm,
    HwLoc,
    SandboxTag,
};

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::chrono::Local;
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::nanvix_sandbox::{
    control_plane_sockaddr_builder,
    gateway_sockaddr_builder,
    linuxd::LinuxDaemon,
    netns::{
        NetnsHandle,
        NetnsInfo,
        NetnsPool,
        NetnsPoolConfig,
        NetnsPoolInitStrategy,
    },
    syscomm::{
        SocketListener,
        SocketType,
        UnboundSocket,
    },
    tcp_port::TcpPort,
    user_vm_sockaddr_builder,
    InitializedSandbox,
    LinuxDaemonArgs,
    RunningSandbox,
    SandboxConfig,
    SnapshotDirHandle,
    UninitializedSandbox,
    UserVmIdentifier,
};
use ::std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Arc,
};
use ::tokio::sync::{
    Mutex,
    MutexGuard,
    RwLock,
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
    /// Shared control plane listener socket (reused across sandboxes for efficiency).
    control_plane_bind_socket: Arc<Mutex<(SocketListener, String, SocketType)>>,
    /// Network namespace pool for different L2 VMs.
    netns_pool: NetnsPool,
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
    l2_enabled: bool,
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

    ///
    /// # Description
    ///
    /// Returns whether the daemon is running with L2 mode enabled.
    ///
    pub fn l2_enabled(&self) -> bool {
        self.l2_enabled
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
    /// This function returns an error if network namespace pool initialization fails or if the
    /// control plane socket cannot be bound.
    ///
    pub async fn new(config: SandboxCacheConfig<T>) -> Result<Arc<Self>> {
        // Only pre-allocate network namespaces when L2 is enabled; otherwise keep it lazy so
        // non-L2 deployments do not try to create netns at startup (which triggers sudo+sysctl).
        let netns_init_strategy: NetnsPoolInitStrategy = if config.l2() {
            match config.netns_pool_size() {
                0 => NetnsPoolInitStrategy::Lazy,
                size => NetnsPoolInitStrategy::Prefill(size),
            }
        } else {
            NetnsPoolInitStrategy::Lazy
        };

        // Build control plane socket address. The control plane socket address is the same for
        // all sandboxes regardless of network namespace, so we initialize it once at cache
        // creation time.
        //
        // In L2 mode, the control plane uses TCP since linuxd runs inside a VM and communicates
        // via the host's VETH interface. We bind to 0.0.0.0:{CONTROL_PLANE_PORT}.
        // In non-L2 mode, we use a Unix socket in the tmp directory.
        let control_plane_bind_sockaddr: String = if config.l2() {
            format!("0.0.0.0:{}", ::config::linuxd::CONTROL_PLANE_PORT)
        } else {
            let (bind_addr, _connect_addr): (String, String) =
                control_plane_sockaddr_builder(config.tmp_directory(), None)?;
            bind_addr
        };

        // Bind control plane socket.
        // In L2 mode, force TCP socket type since we communicate over the network.
        let control_plane_bind_socket_type: SocketType = if config.l2() {
            SocketType::Tcp
        } else {
            config.control_plane_sockaddr_type()
        };
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

        Ok(Arc::new(Self {
            config,
            running_sandboxes: RwLock::new(HashMap::new()),
            tenants: RwLock::new(HashMap::new()),
            control_plane_bind_socket: Arc::new(Mutex::new((
                control_plane_bind_socket,
                control_plane_bind_sockaddr,
                control_plane_bind_socket_type,
            ))),
            netns_pool: NetnsPool::new(
                NetnsPoolConfig::new(
                    ::config::linuxd::GATEWAY_PORT_RANGE_BEGIN,
                    ::config::linuxd::GATEWAY_PORT_RANGE_END,
                )?,
                netns_init_strategy,
            )?,
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
            l2_enabled: self.config.l2(),
        }
    }

    ///
    /// # Description
    ///
    /// Ensures the temporary directory for a tenant exists and returns its path.
    ///
    /// # Arguments
    ///
    /// - `tenant_id`: unique tenant identifier.
    ///
    /// # Returns
    ///
    /// The path to the tenant's temporary directory.
    ///
    fn ensure_tenant_tmp_dir(&self, tenant_id: &str) -> Result<PathBuf> {
        let tenant_tmp_dir: PathBuf = PathBuf::from(self.config.tmp_directory()).join(tenant_id);
        fs::create_dir_all(&tenant_tmp_dir).map_err(|error| {
            let reason: String = format!(
                "failed to create tenant temporary directory (tenant_id={tenant_id}, \
                 tmp_dir={tenant_tmp_dir:?}, error={error:?})"
            );
            error!("ensure_tenant_tmp_dir(): {reason}");
            anyhow::anyhow!(reason)
        })?;
        Ok(tenant_tmp_dir)
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

        // Allocate network namespace handle for this linuxd instance.
        let netns_handle: Option<NetnsHandle> = if self.config.l2() {
            Some(self.netns_pool.allocate().map_err(|error| {
                let reason: String = format!(
                    "failed to allocate netns for linuxd (tenant_id={tenant_id}, error={error:?})"
                );
                error!("get_or_create_linuxd(): {reason}");
                anyhow::anyhow!(reason)
            })?)
        } else {
            None
        };
        let netns_info: Option<NetnsInfo> = netns_handle
            .as_ref()
            .and_then(|netns_handle| netns_handle.netns_info().ok());

        let (_control_plane_bind_sockaddr, control_plane_connect_sockaddr): (String, String) =
            control_plane_sockaddr_builder(self.config.tmp_directory(), netns_info.clone())?;
        let system_vm_sockaddr: String =
            user_vm_sockaddr_builder(self.config.tmp_directory(), tenant_id, self.config.l2())?;
        let linuxd_tmp_dir: PathBuf = self.ensure_tenant_tmp_dir(tenant_id)?;

        // Allocate snapshot dir handle for this linuxd instance.
        let snapshot_dir_handle: Option<SnapshotDirHandle> = if self.config.l2() {
            let linuxd_log_file: PathBuf = PathBuf::from(self.config.log_directory()).join(
                format!("linuxd-l2_{}_{}.log", tenant_id, Local::now().format("%Y-%m-%d_%H-%M-%S")),
            );
            let snapshot_dir: PathBuf =
                linuxd_tmp_dir.join(format!("l2-sysvm-snapshot-{tenant_id}"));

            Some(
                SnapshotDirHandle::new(
                    &snapshot_dir,
                    self.config.l2_snapshot_path(),
                    linuxd_log_file,
                )
                .map_err(|error| {
                    let reason: String = format!(
                        "failed to create snapshot directory handle (tenant_id={tenant_id}, \
                         error={error:?})"
                    );
                    error!("get_or_create_linuxd(): {reason}");
                    anyhow::anyhow!(reason)
                })?,
            )
        } else {
            None
        };

        let linuxd_args: LinuxDaemonArgs<T> = LinuxDaemonArgs::new(
            tenant_id,
            (control_plane_connect_sockaddr, self.config.control_plane_sockaddr_type()),
            (system_vm_sockaddr, self.config.system_vm_sockaddr_type()),
            self.config.hwloc(),
            self.config.linuxd_binary_path().to_string(),
            self.config.toolchain_binary_directory().to_string(),
            self.config.log_directory().to_string(),
            linuxd_tmp_dir.to_string_lossy().into_owned(),
            self.config.l2(),
        );

        // Here we acquire a lock on the control-plane bind socket while holding the write lock
        // on linuxd_instance. There is no risk of deadlocks because the only two other places
        // where we try and lock this socket are:
        //
        // - UninitializedSandbox::initialize() - in the branch where linuxd is not initialized,
        //   but we never hit this branch in multi-process, as we are spawning linuxd here.
        //
        // - InitializedSandbox::start() - when spawning a user VM. Note that a user VM whose
        //   tenant has not been initialized will never make it to start(), so it is not possible
        //   for two tasks to be competing on the linuxd_instance write lock and the control-plane
        //   lock when spawning a user VM for the same tenant.
        let linuxd: Arc<LinuxDaemon> = {
            let mut listener_and_info: MutexGuard<'_, (SocketListener, String, SocketType)> =
                self.control_plane_bind_socket.lock().await;
            match LinuxDaemon::spawn(
                &linuxd_args,
                &mut listener_and_info.0,
                netns_handle,
                snapshot_dir_handle,
            )
            .await
            {
                Ok(linuxd) => Arc::new(linuxd),
                Err(error) => {
                    let reason: String =
                        format!("failed to spawn linuxd (tenant_id={tenant_id}, error={error:?})");
                    error!("get_or_create_linuxd(): {reason}");
                    anyhow::bail!(reason);
                },
            }
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
    /// ## RAII-Managed Resources (Automatic Cleanup)
    ///
    /// - **TcpPort**: Automatically released back to the port allocator when dropped. If gateway
    ///   port allocation succeeds but initialization fails, the port is returned to the pool via
    ///   RAII when the `TcpPort` instance goes out of scope.
    ///
    /// - **NetnsHandle**: Reference count is automatically decremented when dropped. When the last
    ///   handle to a network namespace is dropped, the namespace is returned to the pool for
    ///   reuse. If namespace allocation succeeds but initialization fails, the namespace is
    ///   properly cleaned up via RAII semantics.
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
    /// - RAII-managed resources (TCP ports, network namespaces) are automatically cleaned up and
    ///   can be reallocated.
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

        // Allocate gateway port if on an L2 deployment.
        let mut gateway_l2_port: Option<TcpPort> = None;
        let netns_handle: Option<NetnsHandle> = linuxd.netns_handle();
        if let Some(netns_handle) = &netns_handle {
            let tcp_port: TcpPort = netns_handle.allocate_gateway_port().map_err(|e| {
                let reason: String = format!(
                    "error allocating gateway port (tenant_id={}, program={}, app_name={}, \
                     error={e:?})",
                    tag.tenant_id(),
                    tag.program(),
                    tag.app_name()
                );
                error!("get(): {reason}");
                anyhow::anyhow!(reason)
            })?;
            gateway_l2_port = Some(tcp_port);
        }

        let control_plane_bind_socket: Arc<Mutex<(SocketListener, String, SocketType)>> =
            self.control_plane_bind_socket.clone();
        let uninitialized_sandbox: UninitializedSandbox<T> = UninitializedSandbox::new(
            tag.program(),
            tag.program_args().cloned(),
            self.config.ramfs_filename().map(|s| s.to_string()),
            control_plane_bind_socket,
        )
        .with_netns_handle(netns_handle)
        .with_linuxd(linuxd);

        // Work-out socket addresses. In L2 deployments these addresses depend on the
        // network namespace, so we assign them right after setting up the netns.
        let netns_info: Option<NetnsInfo> = uninitialized_sandbox.netns_info();
        let (control_plane_bind_sockaddr, control_plane_connect_sockaddr): (String, String) =
            control_plane_sockaddr_builder(self.config.tmp_directory(), netns_info.clone())?;
        let user_vm_sockaddr: String = user_vm_sockaddr_builder(
            self.config.tmp_directory(),
            tag.tenant_id(),
            self.config.l2(),
        )?;
        let gateway_sockaddr: String = gateway_sockaddr_builder(
            self.config.tmp_directory(),
            tag.tenant_id(),
            tag.sandbox_id(),
            netns_info.clone(),
            &gateway_l2_port,
        )?;

        let gateway_socket_address: String = gateway_sockaddr.clone();
        let gateway_socket_type: SocketType = self.config.gateway_sockaddr_type();

        // Work-out the temporary directory for this sandbox based on the base temporary
        // directory for the sandbox cache, and the tenant id.
        let sandbox_tmp_dir: PathBuf =
            self.ensure_tenant_tmp_dir(tag.tenant_id()).map_err(|e| {
                let reason: String = format!(
                    "failed to prepare sandbox temporary directory (tenant_id={}, program={}, \
                     app_name={}, error={e:?})",
                    tag.tenant_id(),
                    tag.program(),
                    tag.app_name()
                );
                error!("get(): {reason}");
                anyhow::anyhow!(reason)
            })?;

        let config: SandboxConfig<T> = SandboxConfig::new(
            tag.tenant_id(),
            tag.sandbox_id(),
            (gateway_socket_address.clone(), gateway_socket_type, gateway_l2_port),
            (user_vm_sockaddr.clone(), self.config.system_vm_sockaddr_type()),
            self.config.console_file().map(|s| s.to_string()),
            self.config.hwloc().clone(),
            self.config.kernel_binary_path(),
            self.config.linuxd_binary_path(),
            self.config.uservm_binary_path(),
            self.config.log_directory(),
            Some((control_plane_bind_sockaddr.clone(), self.config.control_plane_sockaddr_type())),
            (control_plane_connect_sockaddr.clone(), self.config.control_plane_sockaddr_type()),
            Some(self.config.toolchain_binary_directory().to_string()),
            Some(sandbox_tmp_dir.to_string_lossy().into_owned()),
            Some(self.config.l2()),
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
            "cleanup summary: running_sandboxes={}, linuxd_instances={}, l2_enabled={}",
            summary.running_sandboxes(),
            summary.linuxd_instances(),
            summary.l2_enabled()
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
            0,
            &format!("{}/kernel.elf", tmp_dir.path()),
            &format!("{}/linuxd.elf", tmp_dir.path()),
            &format!("{}/uservm.elf", tmp_dir.path()),
            &format!("{}/toolchain", tmp_dir.path()),
            &format!("{}/logs", tmp_dir.path()),
            false,
            &format!("{}/snapshot", tmp_dir.path()),
            tmp_dir.path(),
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
    /// - `l2`: Whether to enable L2 mode.
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
        l2: bool,
    ) -> (SandboxCacheConfig<()>, TempTestDir) {
        let tmp_dir: TempTestDir = TempTestDir::new();

        let netns_pool_size: usize = 0;
        let config: SandboxCacheConfig<()> = SandboxCacheConfig::new(
            socket_type,
            socket_type,
            socket_type,
            console_file,
            None,
            hwloc,
            netns_pool_size,
            &format!("{}/kernel.elf", tmp_dir.path()),
            &format!("{}/linuxd.elf", tmp_dir.path()),
            &format!("{}/uservm.elf", tmp_dir.path()),
            &format!("{}/toolchain", tmp_dir.path()),
            &format!("{}/logs", tmp_dir.path()),
            l2,
            &format!("{}/snapshot", tmp_dir.path()),
            tmp_dir.path(),
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
    /// Tests sandbox cache creation with L2 VM configuration.
    ///
    #[tokio::test]
    async fn test_new_l2_mode() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) =
            create_custom_test_config(None, None, SocketType::Unix, true);
        let result: Result<Arc<SandboxCache<()>>> = SandboxCache::new(config).await;
        assert!(result.is_ok());
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
        assert!(config.toolchain_binary_directory().ends_with("/toolchain"));
        assert!(config.log_directory().ends_with("/logs"));
        assert!(!config.l2());
        assert!(config.l2_snapshot_path().ends_with("/snapshot"));
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
            create_custom_test_config(Some(console_file.clone()), None, SocketType::Unix, false);
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
    /// Tests SandboxCacheConfig with L2 enabled.
    ///
    #[test]
    fn test_config_with_l2_enabled() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) =
            create_custom_test_config(None, None, SocketType::Unix, true);
        assert!(config.l2());
    }

    ///
    /// # Description
    ///
    /// Tests SandboxCacheConfig with different socket types.
    ///
    #[test]
    fn test_config_socket_types() {
        let (config, _tmp_dir): (SandboxCacheConfig<()>, TempTestDir) =
            create_custom_test_config(None, None, SocketType::Tcp, false);
        assert_eq!(config.control_plane_sockaddr_type(), SocketType::Tcp);
        assert_eq!(config.gateway_sockaddr_type(), SocketType::Tcp);
        assert_eq!(config.system_vm_sockaddr_type(), SocketType::Tcp);
    }
}
