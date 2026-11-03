// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    InterruptReason,
    InterruptedThread,
};

//==================================================================================================
// Test Helpers
//==================================================================================================

impl InterruptedThread {
    ///
    /// # Description
    ///
    /// Returns the reason why the thread was interrupted.
    ///
    /// # Returns
    ///
    /// A reference to the [`InterruptReason`] of the interrupted thread.
    ///
    pub fn reason(&self) -> &InterruptReason {
        &self.reason
    }
}
