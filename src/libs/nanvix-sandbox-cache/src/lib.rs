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
// Private Modules
//==================================================================================================

mod tag;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    config::SandboxCacheConfig,
    tag::SandboxTag,
};
pub use ::nanvix_sandbox::{
    syscomm,
    HwLoc,
};

#[cfg(feature = "single-process")]
pub use ::nanvix_sandbox::SyscallTable;

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
#[cfg(not(feature = "single-process"))]
use ::nanvix_sandbox::netns::{
    NetnsHandle,
    NetnsInfo,
    NetnsPool,
    NetnsPoolConfig,
    NetnsPoolInitStrategy,
};
use ::nanvix_sandbox::{
    control_plane_sockaddr_builder,
    gateway_sockaddr_builder,
    linuxd::LinuxDaemon,
    syscomm::{
        SocketListener,
        SocketType,
    },
    tcp_port::TcpPort,
    user_vm_sockaddr_builder,
    InitializedSandbox,
    RunningSandbox,
    SandboxConfig,
    UninitializedSandbox,
    UserVmIdentifier,
};
#[cfg(not(feature = "single-process"))]
use ::std::marker::PhantomData;
use ::std::{
    collections::HashMap,
    sync::Arc,
};
use ::syslog::{
    debug,
    error,
    trace,
    warn,
};
use ::tokio::sync::Mutex;

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
/// - `T`: Custom state type for the syscall table. This is passed to system call handlers in
///   single-process mode. Use `()` if no custom state is required.
///
pub struct SandboxCache<T> {
    /// Configuration parameters for all sandboxes.
    config: SandboxCacheConfig<T>,
    /// Registry of all currently running sandboxes indexed by their unique tag.
    running_sandboxes: HashMap<SandboxTag, RunningSandbox>,
    /// Registry of Linux Daemon instances indexed by tenant ID (one per tenant).
    linuxd_instances: HashMap<String, Arc<LinuxDaemon>>,
    /// Reverse index mapping User VM identifiers to their sandbox tags.
    sandbox_index: HashMap<UserVmIdentifier, SandboxTag>,
    /// Shared control plane listener socket (reused across sandboxes for efficiency).
    control_plane_socket: Option<Arc<Mutex<(SocketListener, String, SocketType)>>>,
    /// Network namespace pool for different L2 VMs.
    #[cfg(not(feature = "single-process"))]
    netns_pool: NetnsPool,
    /// Phantom data to maintain the generic type parameter `T` in the structure.
    /// This is required because `T` is only used in single-process mode for the syscall table.
    #[cfg(not(feature = "single-process"))]
    _phantom: PhantomData<T>,
}

impl<T: Sync + Send + Default + 'static> SandboxCache<T> {
    ///
    /// # Description
    ///
    /// Creates a new sandbox cache wrapped in a shared mutex.
    ///
    /// # Parameters
    ///
    /// - `config`: Configuration parameters for the sandbox cache.
    ///
    /// # Returns
    ///
    /// A shared, mutex-protected sandbox cache ready for concurrent access.
    ///
    /// # Errors
    ///
    /// This function returns an error if network namespace pool initialization fails.
    ///
    pub fn new(config: SandboxCacheConfig<T>) -> Result<Arc<Mutex<Self>>> {
        Ok(Arc::new(Mutex::new(Self {
            config,
            running_sandboxes: HashMap::new(),
            linuxd_instances: HashMap::new(),
            sandbox_index: HashMap::new(),
            control_plane_socket: None,
            #[cfg(not(feature = "single-process"))]
            netns_pool: NetnsPool::new(
                NetnsPoolConfig::new(
                    ::config::linuxd::GATEWAY_PORT_RANGE_BEGIN,
                    ::config::linuxd::GATEWAY_PORT_RANGE_END,
                )?,
                NetnsPoolInitStrategy::Lazy,
            )?,
            #[cfg(not(feature = "single-process"))]
            _phantom: PhantomData,
        })))
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
    /// - **LinuxDaemon**: Wrapped in `Arc<LinuxDaemon>` and stored in `linuxd_instances` map
    ///   (indexed by tenant ID). If Linux Daemon spawns successfully but User VM initialization
    ///   fails, the daemon is **kept** in the cache and reused for subsequent sandbox creation
    ///   attempts within the same tenant. This is intentional: the daemon remains operational and
    ///   can service future requests, avoiding the overhead of respawning.
    ///
    /// - **Control Plane Socket**: Wrapped in `Arc<Mutex<(SocketListener, String, SocketType)>>`
    ///   and stored in `control_plane_socket`. Like the Linux Daemon, it is shared across all
    ///   sandboxes for the same tenant. If created but initialization fails, the socket is
    ///   **retained** and reused. The `SocketListener` Drop implementation ensures Unix socket
    ///   files are removed when the last reference is dropped during cache cleanup.
    ///
    /// ## Cache State Guarantees
    ///
    /// - **sandbox_index**: Only updated after **successful** sandbox startup. If initialization
    ///   or startup fails, the User VM identifier is never added to the index, preventing partial
    ///   state from leaking into the cache.
    ///
    /// - **running_sandboxes**: Only updated after **successful** sandbox startup. Failures during
    ///   initialization or startup do not pollute this map.
    ///
    /// - **linuxd_instances**: Updated after **successful** Linux Daemon spawn, even if User VM
    ///   initialization fails later. This allows daemon reuse across retry attempts.
    ///
    /// ## Arc Reference Counting
    ///
    /// The `LinuxDaemon` and control plane socket are wrapped in `Arc` to enable safe sharing:
    /// - One reference is held in `linuxd_instances` or `control_plane_socket`.
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

        // Construct a tag for the sandbox.
        let tag: SandboxTag = SandboxTag::new(tenant_id, program, app_name, program_args);

        // Check if sandbox is in cache.
        match self.running_sandboxes.get(&tag) {
            // Cache hit: sandbox found.
            Some(sandbox) => Ok((
                tag.sandbox_id(),
                sandbox.gateway_socket_info().0.clone(),
                sandbox.gateway_socket_info().1,
            )),
            // Cache miss: sandbox not found.
            None => {
                let uninitialized_sandbox: UninitializedSandbox<T> =
                    UninitializedSandbox::new(tag.program(), tag.program_args().cloned());

                let gateway_l2_port: Option<TcpPort> = None;

                // Work-around gateway_l2_port only being mutated in multi-process mode. The
                // long-term fix would be to properly gate all instances in the sandbox cache where
                // we use TcpPort behind this feature flag.
                #[cfg(not(feature = "single-process"))]
                let mut gateway_l2_port: Option<TcpPort> = gateway_l2_port;

                // Add Linux Daemon instance to sandbox if one exists for the tenant.
                let uninitialized_sandbox: UninitializedSandbox<T> =
                    if let Some(linuxd) = self.linuxd_instances.get(tag.tenant_id()) {
                        #[cfg(not(feature = "single-process"))]
                        let uninitialized_sandbox: UninitializedSandbox<T> = {
                            // Clone ownership of the network namespace from linuxd (pre-existing) to
                            // the new user VM (only in L2-mode).
                            let netns_handle: Option<NetnsHandle> = linuxd.netns_handle();

                            // Allocate new gateway port from the allocator inside the network
                            // namespace.
                            if let Some(netns_handle) = &netns_handle {
                                let tcp_port: TcpPort =
                                    netns_handle.allocate_gateway_port().map_err(|e| {
                                        let reason: String =
                                            format!("error allocating gateway port (error={e:?})");
                                        error!("get(): {reason}");
                                        anyhow::anyhow!(reason)
                                    })?;

                                // Pass ownership of the tcp_port to the outer scope.
                                gateway_l2_port = Some(tcp_port);
                            }

                            uninitialized_sandbox.with_netns_handle(linuxd.netns_handle())
                        };

                        uninitialized_sandbox.with_linuxd(linuxd.clone())
                    } else {
                        // Allocate a network namespace for the new linuxd (and user VM) instance
                        // in L2 deployments.
                        #[cfg(not(feature = "single-process"))]
                        if self.config.l2() {
                            let netns_handle: NetnsHandle =
                                self.netns_pool.allocate().map_err(|e| {
                                    let reason: String =
                                        format!("failed to allocate netns (error={e:?})");
                                    error!("get(): {reason}");
                                    anyhow::anyhow!("{reason}")
                                })?;

                            let tcp_port: TcpPort =
                                netns_handle.allocate_gateway_port().map_err(|e| {
                                    let reason: String =
                                        format!("error allocating gateway port (error={e:?})");
                                    error!("get(): {reason}");
                                    anyhow::anyhow!(reason)
                                })?;

                            // Pass ownership of the tcp_port to the outer scope.
                            gateway_l2_port = Some(tcp_port);

                            // Pass ownership of the netns handle to the sandbox.
                            uninitialized_sandbox.with_netns_handle(Some(netns_handle))
                        } else {
                            uninitialized_sandbox
                        }

                        #[cfg(feature = "single-process")]
                        uninitialized_sandbox
                    };

                // Work-out socket addresses. In L2 deployments these addresses depend on the
                // network namespace, so we assign them right after setting up the netns.
                #[cfg(not(feature = "single-process"))]
                let netns_info: Option<NetnsInfo> = uninitialized_sandbox.netns_info();
                let control_plane_sockaddr: String = (control_plane_sockaddr_builder)(
                    self.config.tmp_directory(),
                    tag.tenant_id(),
                    #[cfg(not(feature = "single-process"))]
                    netns_info.clone(),
                )?;
                let user_vm_sockaddr: String = (user_vm_sockaddr_builder)(
                    self.config.tmp_directory(),
                    tag.tenant_id(),
                    #[cfg(not(feature = "single-process"))]
                    self.config.l2(),
                )?;
                let gateway_sockaddr: String = (gateway_sockaddr_builder)(
                    self.config.tmp_directory(),
                    tag.tenant_id(),
                    tag.sandbox_id(),
                    #[cfg(not(feature = "single-process"))]
                    netns_info.clone(),
                    #[cfg(not(feature = "single-process"))]
                    &gateway_l2_port,
                )?;

                // Add control-plane socket if one exists.
                let uninitialized_sandbox: UninitializedSandbox<T> =
                    if let Some(control_plane_socket) = &self.control_plane_socket {
                        uninitialized_sandbox
                            .with_control_plane_socket(control_plane_socket.clone())
                    } else {
                        uninitialized_sandbox
                    };

                let gateway_socket_address: String = gateway_sockaddr.clone();
                let gateway_socket_type: SocketType = self.config.gateway_sockaddr_type();

                let config: SandboxConfig<T> = SandboxConfig::new(
                    tag.sandbox_id(),
                    (gateway_socket_address.clone(), gateway_socket_type, gateway_l2_port),
                    (user_vm_sockaddr.clone(), self.config.system_vm_sockaddr_type()),
                    self.config.console_file().map(|s| s.to_string()),
                    self.config.hwloc().clone(),
                    self.config.kernel_binary_path(),
                    #[cfg(not(feature = "single-process"))]
                    self.config.linuxd_binary_path(),
                    #[cfg(not(feature = "single-process"))]
                    self.config.uservm_binary_path(),
                    self.config.log_directory(),
                    #[cfg(feature = "single-process")]
                    self.config.syscall_table(),
                    Some((
                        control_plane_sockaddr.clone(),
                        self.config.control_plane_sockaddr_type(),
                    )),
                    Some(self.config.toolchain_binary_directory().to_string()),
                    Some(self.config.tmp_directory().to_string()),
                    Some(self.config.l2()),
                    Some(self.config.l2_snapshot_path().to_string()),
                );

                let uninitialized_sandbox: UninitializedSandbox<T> =
                    uninitialized_sandbox.with_config(config);

                let initialized_sandbox: InitializedSandbox<T> =
                    match uninitialized_sandbox.initialize().await {
                        Ok(sandbox) => sandbox,
                        Err(error) => {
                            error!("get(): failed to initialize sandbox (error={error:?})");
                            return Err(error);
                        },
                    };

                // Update control-plane socket.
                self.control_plane_socket
                    .replace(initialized_sandbox.control_plane_socket_info());

                // Update Linux Daemon instance.
                self.linuxd_instances
                    .insert(tag.tenant_id().to_string(), initialized_sandbox.linuxd());
                self.sandbox_index.insert(tag.sandbox_id(), tag.clone());

                // Run sandbox.
                match initialized_sandbox.start().await {
                    Ok(running_sandbox) => {
                        self.running_sandboxes.insert(tag.clone(), running_sandbox);
                    },
                    Err(error) => {
                        error!("get(): failed to start sandbox (error={error:?})");
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
    /// On success, returns an empty tuple. On failure, returns an error if the User VM
    /// identifier was not found in the cache.
    ///
    pub async fn kill(&mut self, user_vm_id: UserVmIdentifier) -> Result<()> {
        let tag = self
            .sandbox_index
            .get(&user_vm_id)
            .ok_or_else(|| anyhow::anyhow!("user VM instance not found in cache"))?;

        self.kill_internal(&tag.clone()).await
    }

    ///
    /// # Description
    ///
    /// Internal helper to terminate and remove a sandbox by its tag.
    ///
    /// # Parameters
    ///
    /// - `tag`: Tag identifying the sandbox to terminate.
    ///
    /// # Returns
    ///
    /// On success, returns an empty tuple. On failure, returns an error.
    ///
    async fn kill_internal(&mut self, tag: &SandboxTag) -> Result<()> {
        let user_vm_id: UserVmIdentifier = tag.sandbox_id();

        if !self.running_sandboxes.contains_key(tag) {
            warn!("trying to kill user VM that is not in the cache (tag={tag:?})");
            return Ok(());
        }

        if let Some(sandbox) = self.running_sandboxes.remove(tag) {
            sandbox.shutdown().await;
        }

        self.sandbox_index.remove(&user_vm_id);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Performs cleanup by gracefully shutting down all sandboxes and Linux Daemon instances.
    ///
    /// This method shuts down all User VMs first, then terminates all Linux Daemon instances.
    /// It should be called when the daemon is shutting down to ensure proper resource cleanup.
    ///
    pub async fn cleanup(&mut self) {
        debug!("cleaning up sandbox cache");

        // First shutdown all user VMs.
        for (tag, sandbox) in self.running_sandboxes.drain() {
            debug!("cleaning user vm instance (tag={tag:?})");
            sandbox.shutdown().await;
        }

        // Shutdown all linuxd instances.
        for (tenant_id, linuxd_instance) in self.linuxd_instances.iter_mut() {
            debug!("cleaning linuxd instance (tenant_id={tenant_id:?})");
            if let Some(linuxd_instance_mut) = Arc::get_mut(linuxd_instance) {
                linuxd_instance_mut.shutdown().await;
            } else {
                error!("error cleaning-up linuxd instance: not found (tenant_id={tenant_id})");
            }
        }
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
    /// Creates a test configuration for single-process mode.
    ///
    /// # Returns
    ///
    /// A sandbox cache configuration suitable for testing.
    ///
    #[cfg(feature = "single-process")]
    fn create_test_config() -> SandboxCacheConfig<()> {
        let tmp_dir: String = ::std::env::temp_dir().to_string_lossy().to_string();
        SandboxCacheConfig::new(
            SocketType::Unix,
            SocketType::Unix,
            SocketType::Unix,
            None,
            None,
            &format!("{}/kernel.elf", tmp_dir),
            None,
            &format!("{}/toolchain", tmp_dir),
            &format!("{}/logs", tmp_dir),
            false,
            &format!("{}/snapshot", tmp_dir),
            &tmp_dir,
        )
    }

    ///
    /// # Description
    ///
    /// Creates a test configuration for multi-process mode.
    ///
    /// # Returns
    ///
    /// A sandbox cache configuration suitable for testing.
    ///
    #[cfg(not(feature = "single-process"))]
    fn create_test_config() -> SandboxCacheConfig<()> {
        let tmp_dir: String = ::std::env::temp_dir().to_string_lossy().to_string();
        SandboxCacheConfig::new(
            SocketType::Unix,
            SocketType::Unix,
            SocketType::Unix,
            None,
            None,
            &format!("{}/kernel.elf", tmp_dir),
            &format!("{}/linuxd.elf", tmp_dir),
            &format!("{}/uservm.elf", tmp_dir),
            &format!("{}/toolchain", tmp_dir),
            &format!("{}/logs", tmp_dir),
            false,
            &format!("{}/snapshot", tmp_dir),
            &tmp_dir,
        )
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
    /// A sandbox cache configuration suitable for testing.
    ///
    fn create_custom_test_config(
        console_file: Option<String>,
        hwloc: Option<HwLoc>,
        socket_type: SocketType,
        l2: bool,
    ) -> SandboxCacheConfig<()> {
        let tmp_dir: String = ::std::env::temp_dir().to_string_lossy().to_string();

        #[cfg(feature = "single-process")]
        {
            SandboxCacheConfig::new(
                socket_type,
                socket_type,
                socket_type,
                console_file,
                hwloc,
                &format!("{}/kernel.elf", tmp_dir),
                None,
                &format!("{}/toolchain", tmp_dir),
                &format!("{}/logs", tmp_dir),
                l2,
                &format!("{}/snapshot", tmp_dir),
                &tmp_dir,
            )
        }

        #[cfg(not(feature = "single-process"))]
        {
            SandboxCacheConfig::new(
                socket_type,
                socket_type,
                socket_type,
                console_file,
                hwloc,
                &format!("{}/kernel.elf", tmp_dir),
                &format!("{}/linuxd.elf", tmp_dir),
                &format!("{}/uservm.elf", tmp_dir),
                &format!("{}/toolchain", tmp_dir),
                &format!("{}/logs", tmp_dir),
                l2,
                &format!("{}/snapshot", tmp_dir),
                &tmp_dir,
            )
        }
    }

    ///
    /// # Description
    ///
    /// Tests sandbox cache creation with default configuration.
    ///
    #[tokio::test]
    async fn test_new_creates_cache() {
        let config: SandboxCacheConfig<()> = create_test_config();
        let result: Result<Arc<Mutex<SandboxCache<()>>>> = SandboxCache::new(config);
        assert!(result.is_ok());
    }

    ///
    /// # Description
    ///
    /// Tests sandbox cache creation with single-process configuration.
    ///
    #[tokio::test]
    #[cfg(feature = "single-process")]
    async fn test_new_single_process_mode() {
        let config: SandboxCacheConfig<()> = create_test_config();
        let result: Result<Arc<Mutex<SandboxCache<()>>>> = SandboxCache::new(config);
        assert!(result.is_ok());

        let cache: Arc<Mutex<SandboxCache<()>>> = result.unwrap();
        let cache_guard: tokio::sync::MutexGuard<SandboxCache<()>> = cache.lock().await;
        assert_eq!(cache_guard.running_sandboxes.len(), 0);
        assert_eq!(cache_guard.linuxd_instances.len(), 0);
        assert_eq!(cache_guard.sandbox_index.len(), 0);
    }

    ///
    /// # Description
    ///
    /// Tests sandbox cache creation with multi-process configuration.
    ///
    #[tokio::test]
    #[cfg(not(feature = "single-process"))]
    async fn test_new_multi_process_mode() {
        let config: SandboxCacheConfig<()> = create_test_config();
        let result: Result<Arc<Mutex<SandboxCache<()>>>> = SandboxCache::new(config);
        assert!(result.is_ok());

        let cache: Arc<Mutex<SandboxCache<()>>> = result.unwrap();
        let cache_guard: tokio::sync::MutexGuard<SandboxCache<()>> = cache.lock().await;
        assert_eq!(cache_guard.running_sandboxes.len(), 0);
        assert_eq!(cache_guard.linuxd_instances.len(), 0);
        assert_eq!(cache_guard.sandbox_index.len(), 0);
    }

    ///
    /// # Description
    ///
    /// Tests sandbox cache creation with L2 VM configuration.
    ///
    #[tokio::test]
    #[cfg(not(feature = "single-process"))]
    async fn test_new_l2_mode() {
        let config: SandboxCacheConfig<()> =
            create_custom_test_config(None, None, SocketType::Unix, true);
        let result: Result<Arc<Mutex<SandboxCache<()>>>> = SandboxCache::new(config);
        assert!(result.is_ok());
    }

    ///
    /// # Description
    ///
    /// Tests that cleanup properly empties all cache structures.
    ///
    #[tokio::test]
    async fn test_cleanup_empties_cache() {
        let config: SandboxCacheConfig<()> = create_test_config();
        let cache: Arc<Mutex<SandboxCache<()>>> = SandboxCache::new(config).unwrap();

        {
            let mut cache_guard: tokio::sync::MutexGuard<SandboxCache<()>> = cache.lock().await;
            cache_guard.cleanup().await;
            assert_eq!(cache_guard.running_sandboxes.len(), 0);
        }
    }

    ///
    /// # Description
    ///
    /// Tests that kill returns an error for non-existent sandbox.
    ///
    #[tokio::test]
    async fn test_kill_nonexistent_sandbox_fails() {
        let config: SandboxCacheConfig<()> = create_test_config();
        let cache: Arc<Mutex<SandboxCache<()>>> = SandboxCache::new(config).unwrap();

        let mut cache_guard: tokio::sync::MutexGuard<SandboxCache<()>> = cache.lock().await;
        let nonexistent_id: UserVmIdentifier = UserVmIdentifier::new(NONEXISTENT_USER_VM_ID);
        let result: Result<()> = cache_guard.kill(nonexistent_id).await;
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
    #[cfg(feature = "single-process")]
    fn test_config_single_process() {
        let config: SandboxCacheConfig<()> = create_test_config();
        let tmp_dir: String = ::std::env::temp_dir().to_string_lossy().to_string();
        assert_eq!(config.control_plane_sockaddr_type(), SocketType::Unix);
        assert_eq!(config.gateway_sockaddr_type(), SocketType::Unix);
        assert_eq!(config.system_vm_sockaddr_type(), SocketType::Unix);
        assert_eq!(config.kernel_binary_path(), format!("{}/kernel.elf", tmp_dir));
        assert_eq!(config.toolchain_binary_directory(), format!("{}/toolchain", tmp_dir));
        assert_eq!(config.log_directory(), format!("{}/logs", tmp_dir));
        assert!(!config.l2());
        assert_eq!(config.l2_snapshot_path(), format!("{}/snapshot", tmp_dir));
        assert_eq!(config.tmp_directory(), tmp_dir);
    }

    ///
    /// # Description
    ///
    /// Tests SandboxCacheConfig creation and getters for multi-process mode.
    ///
    #[test]
    #[cfg(not(feature = "single-process"))]
    fn test_config_multi_process() {
        let config: SandboxCacheConfig<()> = create_test_config();
        let tmp_dir: String = ::std::env::temp_dir().to_string_lossy().to_string();
        assert_eq!(config.control_plane_sockaddr_type(), SocketType::Unix);
        assert_eq!(config.gateway_sockaddr_type(), SocketType::Unix);
        assert_eq!(config.system_vm_sockaddr_type(), SocketType::Unix);
        assert_eq!(config.kernel_binary_path(), format!("{}/kernel.elf", tmp_dir));
        assert_eq!(config.linuxd_binary_path(), format!("{}/linuxd.elf", tmp_dir));
        assert_eq!(config.uservm_binary_path(), format!("{}/uservm.elf", tmp_dir));
        assert_eq!(config.toolchain_binary_directory(), format!("{}/toolchain", tmp_dir));
        assert_eq!(config.log_directory(), format!("{}/logs", tmp_dir));
        assert!(!config.l2());
        assert_eq!(config.l2_snapshot_path(), format!("{}/snapshot", tmp_dir));
        assert_eq!(config.tmp_directory(), tmp_dir);
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
        let config: SandboxCacheConfig<()> =
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
        let config: SandboxCacheConfig<()> = create_test_config();
        assert!(config.hwloc().is_none());
    }

    ///
    /// # Description
    ///
    /// Tests SandboxCacheConfig with L2 enabled.
    ///
    #[test]
    #[cfg(not(feature = "single-process"))]
    fn test_config_with_l2_enabled() {
        let config: SandboxCacheConfig<()> =
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
        let config: SandboxCacheConfig<()> =
            create_custom_test_config(None, None, SocketType::Tcp, false);
        assert_eq!(config.control_plane_sockaddr_type(), SocketType::Tcp);
        assert_eq!(config.gateway_sockaddr_type(), SocketType::Tcp);
        assert_eq!(config.system_vm_sockaddr_type(), SocketType::Tcp);
    }
}
