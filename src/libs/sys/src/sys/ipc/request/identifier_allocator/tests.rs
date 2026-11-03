// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

impl RequestIdentifierAllocator {
    /// Creates an allocator whose next identifier candidate is `next`.
    ///
    /// Defined here, next to the private `next` field it sets, and widened to the request module
    /// so that its tests can reach it.
    pub(in crate::sys::ipc::request) const fn with_next(next: u32) -> Self {
        Self { next }
    }
}
