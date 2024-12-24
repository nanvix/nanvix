// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod handle;

//==================================================================================================
// Imports
//==================================================================================================

use crate::sandbox::{
    Sandbox,
    SandboxConfig,
    SandboxTag,
};
use ::anyhow::Result;
use ::std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use ::tokio::{
    sync::{
        Mutex,
        MutexGuard,
    },
    time,
    time::Instant,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use self::handle::{
    LockedSandbox,
    SandboxHandle,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A cache of sandboxes.
#[derive(Clone)]
pub struct SandboxCache {
    /// Cached value for the keep alive timeout. This value is also stored in the inner state.
    keep_alive_timeout: Duration,
    /// Inner state of the cache.
    inner: Arc<Mutex<SandboxCacheInner>>,
}

/// Inner state of a sandbox cache.
#[derive(Clone)]
pub struct SandboxCacheInner {
    /// Timeout for keeping sandboxes alive.
    keep_alive_timeout: Duration,
    /// Table of sandboxes.
    sandboxes: Arc<Mutex<SandboxTable>>,
}

//==================================================================================================
// Types
//==================================================================================================

/// Type alias to make clippy happy.
type SandboxTable = HashMap<SandboxTag, (Instant, Arc<Mutex<Sandbox>>)>;

/// Type alias for a locked sandbox table.
type LockedSandboxTable<'a> = MutexGuard<'a, HashMap<SandboxTag, (Instant, Arc<Mutex<Sandbox>>)>>;

/// Type alias for a locked sandbox cache.
type LockedSandboxCache<'a> = MutexGuard<'a, SandboxCacheInner>;

//==================================================================================================

impl SandboxCacheInner {
    ///
    /// # Description
    ///
    /// Creates a new sandbox cache.
    ///
    /// # Parameters
    ///
    /// - `keep_alive_timeout`: Timeout for keeping sandboxes alive.
    ///
    /// # Returns
    ///
    /// A new sandbox cache.
    ///
    pub fn new(keep_alive_timeout: Duration) -> Self {
        Self {
            keep_alive_timeout,
            sandboxes: Arc::new(Mutex::new(HashMap::new())),
        }
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
        &self,
        tag: &SandboxTag,
        config: &SandboxConfig,
    ) -> Result<Arc<Mutex<Sandbox>>> {
        let mut locked_sandboxes: LockedSandboxTable = self.sandboxes.lock().await;

        // Attempt to get the sandbox from the cache.
        if let Some((last_access, sandbox)) = locked_sandboxes.get_mut(tag) {
            // Cache hit, update access time.
            debug!("get(): found sandbox {:?} in cache, last access {:?}", tag, last_access);
            *last_access = Instant::now();
            Ok(sandbox.clone())
        } else {
            // Cache miss, create a new sandbox.
            debug!("get(): creating sandbox {:?}", tag);
            let sandbox: Arc<Mutex<Sandbox>> = Arc::new(Mutex::new(Sandbox::new(config)?));
            locked_sandboxes.insert(tag.clone(), (Instant::now(), sandbox.clone()));
            Ok(sandbox)
        }
    }

    ///
    /// # Description
    ///
    /// Tries to cleanup the cache by evicting sandboxes that have expired.
    ///
    pub async fn try_cleanup(&mut self) {
        let mut expired_sandboxes: Vec<SandboxTag> = Vec::new();

        let mut locked_sandboxes: LockedSandboxTable = self.sandboxes.lock().await;

        // Collect all the sandboxes that have expired.
        let now: Instant = Instant::now();
        for (tag, (last_access, _sandbox)) in locked_sandboxes.iter() {
            if let Ok(_locked_sandbox) = _sandbox.try_lock() {
                if now - *last_access > self.keep_alive_timeout {
                    expired_sandboxes.push(tag.clone());
                }
            }
        }

        // Remove expired sandboxes from the cache.
        for tag in expired_sandboxes {
            debug!("try_cleanup(): evicting sandbox {:?}", tag);
            if locked_sandboxes.remove(&tag).is_none() {
                // This condition is unreachable because we have just collected the expired sandboxes.
                // while holding the lock on the cache of sandboxes.
                unreachable!("attempted to remove sandbox that does not exist (tag={:?})", tag);
            }
        }
    }

    ///
    /// # Description
    ///
    /// Updates the access time of a sandbox.
    ///
    /// # Parameters
    ///
    /// - `tag`: Tag of the sandbox.
    ///
    /// # Returns
    ///
    /// Upon success, the access time of the sandbox is updated. Otherwise, an error is returned
    /// instead.
    ///
    async fn update_access_time(&mut self, tag: &SandboxTag) -> Result<()> {
        // Lock the table of sandboxes and attempt to retrieve the target sandbox.
        let mut locked_sandboxes: LockedSandboxTable = self.sandboxes.lock().await;
        if let Some((last_access, _sandbox)) = locked_sandboxes.get_mut(tag) {
            // Sandbox found, update access time.
            *last_access = Instant::now();
        }
        Ok(())
    }
}

impl SandboxCache {
    ///
    /// # Description
    ///
    /// Creates a new sandbox cache.
    ///
    /// # Parameters
    ///
    /// - `keep_alive_timeout`: Timeout for keeping sandboxes alive.
    ///
    /// # Returns
    ///
    /// A new sandbox cache.
    ///
    pub fn new(keep_alive_timeout: Duration) -> Self {
        Self {
            keep_alive_timeout,
            inner: Arc::new(Mutex::new(SandboxCacheInner::new(keep_alive_timeout))),
        }
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
    pub async fn get(&self, tag: &SandboxTag, config: &SandboxConfig) -> Result<SandboxHandle> {
        let locked_cache: LockedSandboxCache = self.inner.lock().await;
        let sandbox: Arc<Mutex<Sandbox>> = locked_cache.get(tag, config).await?;

        Ok(SandboxHandle::new(tag, sandbox, self.inner.clone()))
    }

    ///
    /// # Description
    ///
    /// Tries to cleanup the cache by evicting sandboxes that have expired.
    ///
    /// # Returns
    ///
    /// Upon success, the cache is cleaned up. Otherwise, an error is returned instead.
    ///
    pub async fn try_cleanup(&self) {
        // Sleep for the keep alive timeout to avoid lock contention.
        // NOTE: we sleep before locking the cache to avoid blocking other threads.
        time::sleep(self.keep_alive_timeout).await;
        // Lock the cache and try to cleanup expired sandboxes.
        let mut locked_cache: LockedSandboxCache = self.inner.lock().await;
        locked_cache.try_cleanup().await;
    }
}
