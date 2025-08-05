// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sandbox::{
    linuxd::LinuxDaemon,
    microvm::Microvm,
    config::SandboxConfig,
    tag::SandboxTag,
};
use ::anyhow::Result;
use ::std::{
    collections::HashMap,
    sync::Arc,
};
use ::tokio::sync::Mutex;

//==================================================================================================
// Structures
//==================================================================================================

/// A cache of sandboxes.
#[derive(Clone)]
pub struct SandboxCache {
    // Members holding the state of the cache.
    /// Main table of sandboxes managed by this nanvixd instance.
    user_vm_instances: HashMap<SandboxTag, Arc<Microvm>>,
    /// Table containing linuxd instances. The key is the tenant id as, for the moment, we deploy
    /// only one linuxd instance per tenant.
    linuxd_instances: HashMap<String, Arc<LinuxDaemon>>,

    // Auxiliary index structures.
    /// Reverse index mapping a sandbox ID to a sandbox tag.
    sandbox_index: HashMap<String, SandboxTag>,
}

impl SandboxCache{
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
                        )?));
                }

                // Spawn the user VM that will connect to the linuxd instance.
                self.user_vm_instances.insert(
                    tag.clone(),
                    Arc::new(Microvm::spawn(
                        sandbox_config.program(),
                        sandbox_config.program_args(),
                        sandbox_config.user_vm_sockaddr(),
                        sandbox_config.console_file(),
                        sandbox_config.hwloc(),
                    )?));
                self.sandbox_index.insert(tag.sandbox_id().to_string(), tag.clone());
            } else {
                let reason: String = format!("sandbox not cached, and no sandbox config provided (tag={tag:?})");
                error!("{reason}");
                return Err(anyhow::anyhow!("{reason}"));
            }
        }

        let user_vm = self.user_vm_instances
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
    pub async fn kill(
        &mut self,
        user_vm_id: String,
    ) -> Result<()> {
        let tag = self.sandbox_index
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
    fn kill_internal(
        &mut self,
        tag: &SandboxTag,
    ) -> Result<()> {
        let user_vm_id = tag.sandbox_id();

        if !self.user_vm_instances.contains_key(tag) {
            warn!("trying to drop sandbox (tag={tag:?}) which is not in the cache");
            return Ok(());
        }

        if let Some(user_vm) = self.user_vm_instances.remove(tag) {
            if Arc::strong_count(&user_vm) != 1 {
                warn!("trying to drop user VM, but there are dangling references to it\
                this may introduce unexpected behaviour");
            }

            // User VM is dropped.
        }

        self.sandbox_index.remove(user_vm_id);

        Ok(())
    }
}
