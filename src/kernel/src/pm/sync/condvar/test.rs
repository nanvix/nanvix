// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::Condvar;
use ::sys::pm::ThreadIdentifier;

//==================================================================================================
// Test Helpers
//==================================================================================================

impl Condvar {
    /// Adds a synthetic waiter for an in-kernel notification-path test.
    pub(crate) fn stage_test_waiter(&self, tid: ThreadIdentifier) {
        self.inner.sleeping.borrow_mut().push_back(tid);
    }

    /// Reports whether a synthetic waiter remains queued after a test notification.
    pub(crate) fn has_test_waiter(&self, tid: ThreadIdentifier) -> bool {
        self.inner
            .sleeping
            .borrow()
            .iter()
            .any(|waiter| *waiter == tid)
    }
}
