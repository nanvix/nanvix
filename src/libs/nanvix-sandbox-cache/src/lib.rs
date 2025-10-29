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
use ::nanvix_sandbox::{
    control_plane_sockaddr_builder,
    gateway_sockaddr_builder,
    linuxd::LinuxDaemon,
    syscomm::{
        SocketListener,
        SocketType,
    },
    tcp_port::{
        TcpPort,
        TcpPortAllocator,
    },
    user_vm_sockaddr_builder,
    InitializedSandbox,
    RunningSandbox,
    SandboxConfig,
    UninitializedSandbox,
    UserVmIdentifier,
};
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
pub struct SandboxCache {
    /// Configuration parameters for all sandboxes.
    config: SandboxCacheConfig,
    /// Registry of all currently running sandboxes indexed by their unique tag.
    running_sandboxes: HashMap<SandboxTag, RunningSandbox>,
    /// Registry of Linux Daemon instances indexed by tenant ID (one per tenant).
    linuxd_instances: HashMap<String, Arc<LinuxDaemon>>,
    /// Reverse index mapping User VM identifiers to their sandbox tags.
    sandbox_index: HashMap<UserVmIdentifier, SandboxTag>,
    /// Shared control plane listener socket (reused across sandboxes for efficiency).
    control_plane_socket: Option<Arc<Mutex<(SocketListener, String, SocketType)>>>,
    /// TCP port allocator for gateway ports in L2 deployment mode.
    tcp_port_allocator: TcpPortAllocator,
}

impl SandboxCache {
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
    pub fn new(config: SandboxCacheConfig) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            config,
            running_sandboxes: HashMap::new(),
            linuxd_instances: HashMap::new(),
            sandbox_index: HashMap::new(),
            control_plane_socket: None,
            tcp_port_allocator: TcpPortAllocator::new(
                ::config::linuxd::GATEWAY_PORT_RANGE_BEGIN,
                ::config::linuxd::GATEWAY_PORT_RANGE_END,
            ),
        }))
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
    /// User VM fails), the allocated TCP port is automatically released via RAII. The Linux Daemon
    /// and control plane socket are reused for subsequent sandbox creation attempts within the
    /// same tenant. The sandbox index is not updated, ensuring no partial state leaks into the cache.
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
                // Allocate a TCP port for the gateway if we are in L2 mode.
                let gateway_l2_port: Option<TcpPort> = if self.config.l2() {
                    match self.tcp_port_allocator.allocate().await {
                        Some(port) => Some(port),
                        None => {
                            let reason: String =
                                "failed to allocate TCP port for gateway".to_string();
                            error!("get(): {reason}");
                            return Err(::anyhow::anyhow!("{reason}"));
                        },
                    }
                } else {
                    None
                };

                // Work-out socket addresses before allocating any resources.
                let control_plane_sockaddr: String = (control_plane_sockaddr_builder)(
                    self.config.tmp_directory(),
                    tag.tenant_id(),
                    self.config.l2(),
                )?;
                let user_vm_sockaddr: String = (user_vm_sockaddr_builder)(
                    self.config.tmp_directory(),
                    tag.tenant_id(),
                    self.config.l2(),
                )?;
                let gateway_sockaddr: String = (gateway_sockaddr_builder)(
                    self.config.tmp_directory(),
                    tag.tenant_id(),
                    tag.sandbox_id(),
                    &gateway_l2_port,
                )?;

                let uninitialized_sandbox: UninitializedSandbox =
                    UninitializedSandbox::new(tag.program(), tag.program_args().cloned());

                // Add Linux Daemon instance to sandbox if one exists for the tenant.
                let uninitialized_sandbox: UninitializedSandbox =
                    if let Some(linuxd) = self.linuxd_instances.get(tag.tenant_id()) {
                        uninitialized_sandbox.with_linuxd(linuxd.clone())
                    } else {
                        uninitialized_sandbox
                    };

                // Add control-plane socket if one exists.
                let uninitialized_sandbox: UninitializedSandbox =
                    if let Some(control_plane_socket) = &self.control_plane_socket {
                        uninitialized_sandbox
                            .with_control_plane_socket(control_plane_socket.clone())
                    } else {
                        uninitialized_sandbox
                    };

                let gateway_socket_address: String = gateway_sockaddr.clone();
                let gateway_socket_type: SocketType = self.config.gateway_sockaddr_type();

                let config: SandboxConfig = SandboxConfig::new(
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
                );

                let uninitialized_sandbox: UninitializedSandbox =
                    uninitialized_sandbox.with_config(config);

                let initialized_sandbox: InitializedSandbox =
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
