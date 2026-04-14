// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Platform-agnostic facade over the frame allocator. The inner state
//! type lives in [`crate::hal::platform::frame::Inner`] (per platform);
//! this module owns it directly (no `Rc<RefCell<_>>` here — the
//! allocator is single-owner: the upool consumes it during init and
//! never shares the handle).

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    mem::{
        FrameAddress,
        PageAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
    },
    platform::frame::Inner,
};
use ::sys::error::Error;

//==================================================================================================
// Frame Allocator
//==================================================================================================

#[derive(Debug)]
pub struct FrameAllocator(Inner);

impl FrameAllocator {
    /// Initializes the underlying platform frame allocator.
    pub fn init() -> Result<Self, Error> {
        Ok(Self(Inner::new()?))
    }

    pub fn alloc(&mut self) -> Result<FrameAddress, Error> {
        self.0.alloc()
    }

    pub fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {
        self.0.free(frame)
    }

    pub fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        self.0.book(phys_addr)
    }

    pub fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error> {
        self.0.alloc_range(region)
    }
}
