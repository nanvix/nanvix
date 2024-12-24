// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    cache::{
        LockedSandboxCache,
        SandboxCacheInner,
        SandboxTag,
    },
    sandbox::Sandbox,
};
use ::anyhow::Result;
use ::std::sync::Arc;
use ::tokio::sync::{
    Mutex,
    MutexGuard,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A handle to a sandbox.
pub struct SandboxHandle(Option<SandboxHandleInner>);

/// Inner state of a sandbox handle.
struct SandboxHandleInner {
    /// Tag of the sandbox used for identification.
    tag: SandboxTag,
    /// Sandbox.
    sandbox: Arc<Mutex<Sandbox>>,
    /// Back reference to the cache used for updating cache statics when the handle is dropped.
    cache: Arc<Mutex<SandboxCacheInner>>,
}

//==================================================================================================
// Types
//==================================================================================================

/// Type alias for a locked sandbox.
pub type LockedSandbox<'a> = MutexGuard<'a, Sandbox>;

//==================================================================================================
// Implementations
//==================================================================================================

impl SandboxHandleInner {
    ///
    /// # Description
    ///
    /// Gets the underlying sandbox.
    ///
    /// # Returns
    ///
    /// A reference to the underlying sandbox.
    ///
    async fn get_sandbox(&mut self) -> Result<LockedSandbox> {
        let mut locked_sandbox: LockedSandbox = self.sandbox.lock().await;
        locked_sandbox.load(self.tag.program()).await?;
        Ok(locked_sandbox)
    }
}

impl SandboxHandle {
    ///
    /// # Description
    ///
    /// Creates a new handle to a sandbox.
    ///
    /// # Parameters
    ///
    /// - `tag`: Tag of the sandbox.
    /// - `sandbox`: Sandbox.
    /// - `cache`: Cache.
    ///
    ///
    /// # Returns
    ///
    /// A new handle to a sandbox.
    ///
    pub fn new(
        tag: &SandboxTag,
        sandbox: Arc<Mutex<Sandbox>>,
        cache: Arc<Mutex<SandboxCacheInner>>,
    ) -> Self {
        Self(Some(SandboxHandleInner {
            tag: tag.clone(),
            sandbox,
            cache,
        }))
    }

    ///
    ///
    /// # Description
    ///
    /// Gets the underlying sandbox.
    ///
    /// # Returns
    ///
    /// Upon success, a reference to the underlying sandbox is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub async fn get_sandbox(&mut self) -> Result<LockedSandbox> {
        match self.0 {
            Some(ref mut inner) => inner.get_sandbox().await,
            None => {
                // This condition is unreachable because inner state should be dropped only when the
                // handle is dropped.
                unreachable!("sandbox handle was dropped while still in use");
            },
        }
    }
}

impl Drop for SandboxHandle {
    fn drop(&mut self) {
        // Check if we have not yet dropped the inner state.
        if let Some(inner) = self.0.take() {
            // Spawn a task to lazily update cache statistics and drop the inner state.
            tokio::spawn(async move {
                let mut cache: LockedSandboxCache = inner.cache.lock().await;
                // Update access time of the sandbox and check for errors.
                if let Err(e) = cache.update_access_time(&inner.tag).await {
                    warn!("failed to update sandbox access time (error={:?})", e);
                }
            });
        }
    }
}
