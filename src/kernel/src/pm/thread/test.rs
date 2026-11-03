// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::ThreadRef;

//==================================================================================================
// Test Helpers
//==================================================================================================

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
