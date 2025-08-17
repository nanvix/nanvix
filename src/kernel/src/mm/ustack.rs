// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::PageAligned;
use ::alloc::fmt;
use ::config::memory_layout::USER_STACK_SIZE;
use ::sys::mm::{
    Address,
    VirtualAddress,
};

//==================================================================================================
// User Stack
//==================================================================================================

///
/// # Description
///
/// A structure that represents a user stack.
///
pub struct UserStack {
    /// Base address.
    base: PageAligned<VirtualAddress>,
}

impl UserStack {
    pub fn new(base: PageAligned<VirtualAddress>) -> Self {
        Self { base }
    }

    ///
    /// # Description
    ///
    /// Returns the size of the target stack.
    ///
    /// # Returns
    ///
    /// The size of the target stack.
    ///
    pub fn size(&self) -> usize {
        USER_STACK_SIZE
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the target stack.
    ///
    /// # Returns
    ///
    /// The base address of the target stack.
    ///
    /// # Notes
    ///
    /// As sacks grow downwards, the base address is the highest address of the stack.
    ///
    pub fn base(&self) -> PageAligned<VirtualAddress> {
        self.base
    }

    ///
    /// # Description
    ///
    /// Returns the top address of the target stack.
    ///
    /// # Returns
    ///
    /// The top address of the target stack.
    ///
    /// # Notes
    ///
    /// As stacks grow downwards, the top address is the lowest address of the stack.
    ///
    pub fn top(&self) -> PageAligned<VirtualAddress> {
        PageAligned::from_raw_value(self.base.into_raw_value() + self.size()).unwrap()
    }
}

impl fmt::Debug for UserStack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UserStack {{ base: {:?}, top: {:?}, size={:?} }}",
            self.base,
            self.top(),
            self.size()
        )
    }
}
