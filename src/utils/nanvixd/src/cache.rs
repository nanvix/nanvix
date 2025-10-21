// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    config::GATEWAY_CONNECT_TIMEOUT,
    sandbox::{
        config::SandboxConfig,
        linuxd::LinuxDaemon,
        tag::SandboxTag,
        uservm::UserVm,
    },
};
use ::anyhow::Result;
use ::std::{
    collections::HashMap,
    str::FromStr,
    sync::Arc,
};
use ::syscomm::{
    SocketListener,
    SocketType,
    UnboundSocket,
};
use ::syslog::{
    debug,
    error,
    trace,
    warn,
};
use ::tokio::{
    sync::Mutex,
    time::Instant,
};
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

/// A cache of sandboxes.
pub struct SandboxCache {
    // Members holding the state of the cache.
    /// Main table of sandboxes managed by this nanvixd instance.
    user_vm_instances: HashMap<SandboxTag, Arc<UserVm>>,
    /// Table containing linuxd instances. The key is the tenant id as, for the moment, we deploy
    /// only one linuxd instance per tenant.
    linuxd_instances: HashMap<String, Arc<LinuxDaemon>>,
    // Auxiliary index structures.
    /// Reverse index mapping a sandbox ID to a sandbox tag.
    sandbox_index: HashMap<UserVmIdentifier, SandboxTag>,

    // Control-plane members.
    /// Listener socket on the control-plane address. Right now each different linuxd and user VM
    /// instances have their own control-plane socket.
    control_plane_listener: Option<SocketListener>,
}

impl SandboxCache {
    ///
    /// # Description
    ///
    /// Creates a new sandbox cache protected by a mutex.
    ///
    /// # Parameters
    ///
    /// - `keep_alive_timeout`: Timeout for keeping sandboxes alive.
    ///
    /// # Returns
    ///
    /// A new sandbox cache guarded by a mutex.
    ///
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            user_vm_instances: HashMap::new(),
            linuxd_instances: HashMap::new(),
            sandbox_index: HashMap::new(),
            control_plane_listener: None,
        }))
    }

    ///
    /// # Description
    ///
    /// Gets a sandbox from the cache. If the sandbox is not in the cache, it is created.
    ///
    /// # Parameters
    ///
    /// - `tag`: Tag of the sandbox.
    /// - `config`: Configuration of the sandbox.
    /// - `tmp_directory`: Temporary directory for ephemeral sockets.
    ///
    /// # Returns
    ///
    /// A reference to the sandbox.
    ///
    pub async fn get(
        &mut self,
        tag: &SandboxTag,
        config: Option<SandboxConfig>,
        tmp_directory: String,
    ) -> Result<Arc<UserVm>> {
        trace!("get(): {tag:?}, {config:?}, {tmp_directory:?}");

        if !self.user_vm_instances.contains_key(tag) {
            // Cache miss.
            if let Some(sandbox_config) = config {
                // Start control-plane listener socket lazily.
                if self.control_plane_listener.is_none() {
                    let control_plane_sockaddr: &str = sandbox_config.control_plane_sockaddr();
                    let control_plane_socket_type: SocketType =
                        match sandbox_config.control_plane_sockaddr_type().parse() {
                            Ok(socket_type) => socket_type,
                            Err(e) => {
                                error!(
                                    "invalid control-plane socket type (value={} error={e:?})",
                                    sandbox_config.control_plane_sockaddr_type()
                                );
                                return Err(::anyhow::anyhow!("invalid control-plane socket type"));
                            },
                        };

                    let unbound_socket: UnboundSocket =
                        UnboundSocket::new(control_plane_socket_type);
                    let control_plane_listener: SocketListener = match unbound_socket
                        .bind(control_plane_sockaddr.to_string())
                        .await
                    {
                        Ok(listener) => listener,
                        Err(e) => {
                            error!(
                                "failed to bind control-plane listening socket \
                                 (address={control_plane_sockaddr}, error={e:?})"
                            );
                            return Err(anyhow::anyhow!(
                                "failed to bind control-plane listening socket"
                            ));
                        },
                    };

                    self.control_plane_listener = Some(control_plane_listener);
                }

                let control_plane_listener: &mut SocketListener = self
                    .control_plane_listener
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("control-plane listener is none"))?;

                debug!("creating sandbox {tag:?}");
                if !self.linuxd_instances.contains_key(tag.tenant_id()) {
                    // If this is the first user VM we deploy for this tenant, we first need to
                    // deploy an instance of linuxd.
                    self.linuxd_instances.insert(
                        tag.tenant_id().to_string(),
                        Arc::new(
                            LinuxDaemon::spawn(
                                sandbox_config.control_plane_sockaddr(),
                                sandbox_config.user_vm_sockaddr(),
                                sandbox_config.hwloc(),
                                sandbox_config.binary_directory(),
                                sandbox_config.toolchain_binary_directory(),
                                sandbox_config.log_directory(),
                                control_plane_listener,
                                sandbox_config.l2(),
                                tmp_directory.clone(),
                            )
                            .await?,
                        ),
                    );
                }

                let gateway_sockaddr: String = sandbox_config.gateway_sockaddr().to_string();
                let unbound_gateway_socket: UnboundSocket = UnboundSocket::new(
                    SocketType::from_str(sandbox_config.gateway_sockaddr_type())?,
                );

                // Spawn the user VM that will connect to the linuxd instance.
                self.user_vm_instances.insert(
                    tag.clone(),
                    Arc::new(
                        UserVm::spawn(
                            tag.clone(),
                            // Pass ownership of the sandbox config, including the TCP port if
                            // allocated, to the user VM so that we bind their lifetimes.
                            sandbox_config,
                            control_plane_listener,
                        )
                        .await?,
                    ),
                );

                // Attempt to connect to the gateway socket.
                let now: Instant = Instant::now();
                loop {
                    match unbound_gateway_socket
                        .clone()
                        .connect(gateway_sockaddr.clone())
                        .await
                    {
                        Ok(_stream) => {
                            // Connection successful.
                            break;
                        },
                        Err(_e) => {
                            // Connection failed. Sleep a bit and retry.
                            if now.elapsed().as_secs() > GATEWAY_CONNECT_TIMEOUT.as_secs() {
                                let reason: String = format!(
                                    "failed to connect to gateway socket \
                                     (address={gateway_sockaddr})"
                                );
                                error!("get(): {reason}");
                                return Err(anyhow::anyhow!("{reason}"));
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                        },
                    }
                }

                debug!(
                    "gateway socket file appeared after {:?} (path={:?})",
                    now.elapsed(),
                    &gateway_sockaddr
                );

                self.sandbox_index.insert(tag.sandbox_id(), tag.clone());
            } else {
                let reason: String =
                    format!("sandbox not cached, and no sandbox config provided (tag={tag:?})");
                error!("{reason}");
                return Err(anyhow::anyhow!("{reason}"));
            }
        }

        let user_vm = self
            .user_vm_instances
            .get(tag)
            .ok_or_else(|| anyhow::anyhow!("user VM instance not found in cache"))?;

        Ok(user_vm.clone())
    }

    ///
    /// # Description
    ///
    /// Drops a sandbox from the cache (and kills the underlying process).
    ///
    /// # Parameters
    ///
    /// - `user_vm_id`: ID of the user VM to be dropped.
    ///
    /// # Returns
    ///
    /// A reference to the sandbox.
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
    /// Drops a sandbox from the cache (and kills the underlying process).
    ///
    /// # Parameters
    ///
    /// - `tag`: Tag of the sandbox.
    ///
    /// # Returns
    ///
    /// A reference to the sandbox.
    ///
    async fn kill_internal(&mut self, tag: &SandboxTag) -> Result<()> {
        let user_vm_id = tag.sandbox_id();

        if !self.user_vm_instances.contains_key(tag) {
            warn!("trying to kill user VM that is not in the cache (tag={tag:?})");
            return Ok(());
        }

        if let Some(mut user_vm) = self.user_vm_instances.remove(tag) {
            if let Some(user_vm_mut) = Arc::get_mut(&mut user_vm) {
                user_vm_mut.shutdown().await;
            } else {
                error!("error shutting down user VM: cannot get mut (tag={tag:?})");
            }
        }

        self.sandbox_index.remove(&user_vm_id);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Stop all instances in the cache.
    ///
    /// # Returns
    ///
    /// On success empty is returned. On failure an error is returned instead.
    ///
    pub async fn cleanup(&mut self) {
        debug!("cleaning up sandbox cache");

        // First shutdown all user VMs.
        for (tag, user_vm_instance) in self.user_vm_instances.iter_mut() {
            debug!("cleaning user vm instance (tag={tag:?})");
            if let Some(user_vm_instance_mut) = Arc::get_mut(user_vm_instance) {
                debug!("sending shutdown message to user vm (tag={tag:?})");
                user_vm_instance_mut.shutdown().await;
            } else {
                error!("error cleaning-up user vm instance: not found (tag={tag:?})");
            }
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
