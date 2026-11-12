// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    ThreadManager,
    ThreadRef,
};

//==================================================================================================
// Test Helpers
//==================================================================================================

impl ThreadManager {
    ///
    /// # Description
    ///
    /// Returns the number of threads that hold execution capacity.
    ///
    /// # Return Value
    ///
    /// This function returns the number of created threads that were not reaped yet, including the
    /// bootstrap kernel thread.
    ///
    pub(crate) fn live_count(&self) -> usize {
        self.live_count
    }
}

impl<'a> ThreadRef<'a> {
    ///
    /// # Description
    ///
    /// Returns whether the referenced thread is detached.
    ///
    /// # Return Value
    ///
    /// This function returns `true` if the thread is detached, `false` otherwise.
    ///
    pub fn is_detached(&self) -> bool {
        match self {
            ThreadRef::Ready(thread) => thread.is_detached(),
            ThreadRef::Running(thread) => thread.is_detached(),
            ThreadRef::Sleeping(thread) => thread.is_detached(),
            ThreadRef::Interrupted(thread) => thread.is_detached(),
            ThreadRef::Zombie(thread) => thread.is_detached(),
        }
    }
}
