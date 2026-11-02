// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

impl RequestState {
    /// Creates an unclaimed request state with a test-specific stash capacity.
    ///
    /// Defined here, next to the private fields it sets, and widened to the request module so that
    /// its tests can reach it.
    ///
    /// # Parameters
    ///
    /// - `stash_capacity`: Maximum number of out-of-order responses retained by the state.
    ///
    /// # Returns
    ///
    /// A request state whose stash has already reserved `stash_capacity` entries.
    pub(in crate::sys::ipc::request) fn with_stash_capacity(stash_capacity: usize) -> Self {
        let mut state: Self = Self::new();
        state.stash = VecDeque::with_capacity(stash_capacity);
        state.stash_capacity = stash_capacity;
        state
    }

    /// Replaces the next request-identifier candidate for a test.
    ///
    /// # Parameters
    ///
    /// - `next`: Raw identifier that the allocator should consider next.
    pub(in crate::sys::ipc::request) fn set_next_identifier(&mut self, next: u32) {
        self.allocator = RequestIdentifierAllocator::with_next(next);
    }
}
