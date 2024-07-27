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
    mm::phys::KernelFrame,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug)]
pub struct KernelPage {
    /// Underlying kernel frame.
    kframe: KernelFrame,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl KernelPage {
    ///
    /// # Description
    ///
    /// Instantiate a  kernel page.
    ///
    /// # Parameters
    ///
    /// - `kframe`: Underlying kernel frame.
    ///
    /// # Returns
    ///
    /// A new kernel page.
    ///
    pub fn new(kframe: KernelFrame) -> Self {
        Self { kframe }
    }

    ///
    /// # Description
    ///
    /// Get the base address of the target kernel page.
    ///
    /// # Returns
    ///
    /// The page address of the target kernel page.
    ///
    pub fn base(&self) -> PageAddress {
        // TODO: rename this function to `page_address()`.
        PageAddress::new(
            self.kframe
                .base()
                .into_page_address()
                .into_virtual_address(),
        )
    }

    ///
    /// # Description
    ///
    /// Get the frame address of the target kernel page.
    ///
    /// # Returns
    ///
    /// The frame address of the target kernel page.
    ///
    pub fn frame_address(&self) -> FrameAddress {
        self.kframe.base()
    }
}
