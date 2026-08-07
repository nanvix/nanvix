// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::state::REQUEST_STATES;

//==================================================================================================
// Structures
//==================================================================================================

/// Holds the request-state mutex across a process duplication.
pub struct RequestStateForkGuard {
    _private: (),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RequestStateForkGuard {
    /// Locks request state before duplicating the calling process.
    ///
    /// # Safety
    ///
    /// Signal delivery must be blocked, and the caller must not access request state until this
    /// guard is dropped in both the parent and child.
    pub unsafe fn acquire() -> Self {
        let guard = REQUEST_STATES.lock();
        core::mem::forget(guard);
        Self { _private: () }
    }
}

impl Drop for RequestStateForkGuard {
    fn drop(&mut self) {
        // SAFETY: `acquire()` forgot the unique guard that locked this process's copy. After fork,
        // the parent and child each unlock their own private copy exactly once.
        unsafe {
            REQUEST_STATES.force_unlock();
        }
    }
}
