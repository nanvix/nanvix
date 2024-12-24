// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        FrameAddress,
        PageAddress,
    },
    mm::phys::UserFrame,
};

//==================================================================================================
// Structures
//==================================================================================================

/// A type that represents a user page attached to a virtual memory space.
pub struct AttachedUserPage {
    /// Virtual address to which the page is attached to.
    addr: PageAddress,
    /// Underlying user page.
    frame: UserFrame,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl AttachedUserPage {
    /// Initializes a new attached user page.
    pub fn new(addr: PageAddress, frame: UserFrame) -> Self {
        Self { addr, frame }
    }

    /// Returns the address to which the page is attached to.
    pub fn vaddr(&self) -> PageAddress {
        self.addr
    }

    pub fn frame_address(&self) -> FrameAddress {
        self.frame.address()
    }

    ///
    /// # Description
    ///
    /// Converts the target user page into owned parts.
    ///
    /// # Returns
    ///
    /// A tuple containing the page address and the user frame.
    ///
    pub fn into_owned_parts(self) -> (PageAddress, UserFrame) {
        (self.addr, self.frame)
    }
}
