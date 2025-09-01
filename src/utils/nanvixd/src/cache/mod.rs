// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sandbox::{
    config::SandboxConfig,
    linuxd::LinuxDaemon,
    microvm::Microvm,
    tag::SandboxTag,
};
use ::anyhow::Result;
use ::mio::{
    Interest,
    Poll,
    Token,
};
use ::std::{
    collections::HashMap,
    sync::Arc,
};
use ::syscomm::{
    Socket,
    SocketListener,
    SocketType,
};
use ::tokio::sync::Mutex;
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Constants
//==================================================================================================

/// This is the token we use to register the control-plane listener socket in the poll structure.
/// Right now, we only keep this socket in the poll.
const CONTROL_PLANE_LISTENER_TOKEN: Token = Token(0);

//==================================================================================================
// Structures
//==================================================================================================

/// A cache of sandboxes.
pub struct SandboxCache {
    // Members holding the state of the cache.
    /// Main table of sandboxes managed by this nanvixd instance.
    user_vm_instances: HashMap<SandboxTag, Arc<Microvm>>,
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
    /// Poll structure to support accepting connections into the control-plane with a timeout.
    control_plane_poll: Option<Poll>,
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
            control_plane_poll: None,
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
    ///
    /// # Returns
    ///
    /// A reference to the sandbox.
    ///
    pub async fn get(
        &mut self,
        tag: &SandboxTag,
        config: Option<&SandboxConfig>,
    ) -> Result<Arc<Microvm>> {
        if !self.user_vm_instances.contains_key(tag) {
            // Cache miss.
            if let Some(sandbox_config) = config {
                // Start control-plane listener socket lazily.
                // FIXME(#764): make SocketType configurable.
                if self.control_plane_listener.is_none() {
                    let control_plane_sockaddr = sandbox_config.control_plane_sockaddr();
                    let mut control_plane_listener =
                        match Socket::bind(SocketType::Unix, control_plane_sockaddr.to_string()) {
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

                    // Add control-plane socket to a poll structure so that we can accept
                    // connections with a timeout.
                    let poll: Poll = Poll::new().map_err(|e| {
                        let reason: String =
                            format!("failed to create control-plane poll (error={e:?})");
                        error!("{reason}");
                        anyhow::anyhow!("{reason}")
                    })?;
                    poll.registry()
                        .register(
                            &mut control_plane_listener,
                            CONTROL_PLANE_LISTENER_TOKEN,
                            Interest::READABLE,
                        )
                        .map_err(|e| {
                            let reason: String =
                                format!("failed to create control-plane poll (error={e:?})");
                            error!("{reason}");
                            anyhow::anyhow!("{reason}")
                        })?;

                    self.control_plane_listener = Some(control_plane_listener);
                    self.control_plane_poll = Some(poll);
                }

                let control_plane_listener = self
                    .control_plane_listener
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("control-plane listener is none"))?;

                let control_plane_poll = self
                    .control_plane_poll
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("control-plane poll is none"))?;

                debug!("creating sandbox {tag:?}");
                if !self.linuxd_instances.contains_key(tag.tenant_id()) {
                    // If this is the first user VM we deploy for this tenant, we first need to
                    // deploy an instance of linuxd.
                    self.linuxd_instances.insert(
                        tag.tenant_id().to_string(),
                        Arc::new(LinuxDaemon::spawn(
                            sandbox_config.control_plane_sockaddr(),
                            sandbox_config.user_vm_sockaddr(),
                            sandbox_config.gateway_sockaddr(),
                            sandbox_config.hwloc(),
                            sandbox_config.binary_directory(),
                            sandbox_config.toolchain_binary_directory(),
                            control_plane_listener,
                            control_plane_poll,
                        )?),
                    );
                }

                // Spawn the user VM that will connect to the linuxd instance.
                self.user_vm_instances.insert(
                    tag.clone(),
                    Arc::new(Microvm::spawn(
                        tag.sandbox_id(),
                        sandbox_config.program(),
                        sandbox_config.program_args(),
                        sandbox_config.user_vm_sockaddr(),
                        sandbox_config.control_plane_sockaddr(),
                        sandbox_config.console_file(),
                        sandbox_config.hwloc(),
                        sandbox_config.binary_directory(),
                        control_plane_listener,
                        control_plane_poll,
                    )?),
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

        self.kill_internal(&tag.clone())
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
    fn kill_internal(&mut self, tag: &SandboxTag) -> Result<()> {
        let user_vm_id = tag.sandbox_id();

        if !self.user_vm_instances.contains_key(tag) {
            warn!("trying to drop sandbox (tag={tag:?}) which is not in the cache");
            return Ok(());
        }

        if let Some(user_vm) = self.user_vm_instances.remove(tag) {
            if Arc::strong_count(&user_vm) != 1 {
                warn!(
                    "trying to drop user VM, but there are dangling references to itthis may \
                     introduce unexpected behaviour"
                );
            }

            // User VM is dropped.
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
        // TODO: for the time being only linuxd instances support being gracefully shutdown. User
        // VMs are missing a control-plane socket.
        for (tenant_id, linuxd_instance) in self.linuxd_instances.iter_mut() {
            if let Some(linuxd_instance_mut) = Arc::get_mut(linuxd_instance) {
                if let Err(e) = linuxd_instance_mut.shutdown().await {
                    // FIXME: convert to error once linuxd supports a graceful shutdown.
                    debug!("error cleaning-up linuxd instance for tenant {tenant_id}: {e:?}");
                } else {
                    debug!("cleaned-up linuxd instance for tenant {tenant_id}");
                }
            } else {
                error!(
                    "error cleaning-up linuxd instance for tenant {tenant_id}: instance not found"
                );
            }
        }
    }
}
