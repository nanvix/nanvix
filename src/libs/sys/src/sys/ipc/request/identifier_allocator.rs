// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::identifier::RequestIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

pub(super) struct RequestIdentifierAllocator {
    next: u32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RequestIdentifierAllocator {
    pub(super) const fn new() -> Self {
        Self { next: 1 }
    }

    #[cfg(test)]
    pub(super) const fn with_next(next: u32) -> Self {
        Self { next }
    }

    pub(super) fn allocate(&mut self) -> RequestIdentifier {
        let identifier: RequestIdentifier = RequestIdentifier::from_raw(self.next);
        self.next = self.next.wrapping_add(1);
        if self.next == RequestIdentifier::NONE.raw() {
            self.next = 1;
        }
        identifier
    }
}
