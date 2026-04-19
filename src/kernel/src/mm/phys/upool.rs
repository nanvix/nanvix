// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::FrameAddress,
    mm::phys::frame,
};
use ::alloc::vec::Vec;
use ::core::mem::ManuallyDrop;
use ::sys::error::Error;

//==================================================================================================
// User Frame
//==================================================================================================

///
/// # Description
///
/// A type that represents a user frame.
///
#[derive(Debug)]
pub struct UserFrame {
    /// Frame address.
    addr: FrameAddress,
}

impl UserFrame {
    ///
    /// # Description
    ///
    /// Instantiates a user frame.
    ///
    /// # Parameters
    ///
    /// - `addr`: Frame address.
    ///
    /// # Returns
    ///
    /// A user frame.
    ///
    pub fn new(addr: FrameAddress) -> Self {
        Self { addr }
    }

    ///
    /// # Description
    ///
    /// Returns the physical address of the target user frame.
    ///
    /// # Returns
    ///
    /// The physical address of the target user frame.
    ///
    pub fn address(&self) -> FrameAddress {
        self.addr
    }

    ///
    /// # Description
    ///
    /// Consumes the user frame without freeing the underlying physical frame.
    ///
    /// # Returns
    ///
    /// The frame address.
    ///
    pub fn leak(self) -> FrameAddress {
        let this: ManuallyDrop<Self> = ManuallyDrop::new(self);
        this.addr
    }
}

impl Drop for UserFrame {
    fn drop(&mut self) {
        if let Err(e) = frame::free(self.addr) {
            error!("failed to free user frame: {:?}", e);
        }
    }
}

//==================================================================================================
// User Frame Pool
//==================================================================================================

///
/// # Description
///
/// Thin facade over the module-level [`frame`](super::frame) allocator. Exists as a distinct type
/// so user-frame allocation has its own entry point ([`Upool::alloc_many`] returning [`UserFrame`]).
///
#[derive(Debug)]
pub struct Upool;

impl Upool {
    ///
    /// # Description
    ///
    /// Instantiates a user frame pool.
    ///
    /// # Returns
    ///
    /// A user frame pool.
    ///
    pub fn new() -> Self {
        Self
    }

    pub fn alloc_many(&mut self, nframes: usize) -> Result<Vec<UserFrame>, Error> {
        trace!("nframes={nframes:?}");

        let mut uframes: Vec<UserFrame> = Vec::with_capacity(nframes);
        for _ in 0..nframes {
            match frame::alloc() {
                Ok(addr) => uframes.push(UserFrame::new(addr)),
                // Rollback is automatic: dropping `uframes` frees all allocated frames.
                Err(error) => return Err(error),
            }
        }

        Ok(uframes)
    }
}
