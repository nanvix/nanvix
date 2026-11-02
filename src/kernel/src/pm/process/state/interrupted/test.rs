// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::InterruptedProcess;
use crate::pm::thread::InterruptReason;
use ::sys::pm::ThreadIdentifier;

//==================================================================================================
// Test Helpers
//==================================================================================================

impl InterruptedProcess {
    ///
    /// # Description
    ///
    /// Returns the interrupt reason recorded for the interrupted thread identified by `tid`.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the thread to inspect.
    ///
    /// # Returns
    ///
    /// The [`InterruptReason`] of the matching interrupted thread, or [`None`] if no interrupted
    /// thread with that identifier exists.
    ///
    pub(in crate::pm::process::state) fn thread_reason(
        &self,
        tid: ThreadIdentifier,
    ) -> Option<&InterruptReason> {
        self.interrupted_threads
            .iter()
            .find(|thread| thread.id() == tid)
            .map(|thread| thread.reason())
    }
}
