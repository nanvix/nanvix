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
                Err(error) => {
                    // Roll back: free every frame that was already allocated.
                    for f in uframes {
                        if let Err(e) = frame::free(f.address()) {
                            error!("rollback free failed: {:?}", e);
                        }
                    }
                    return Err(error);
                },
            }
        }

        Ok(uframes)
    }

    ///
    /// # Description
    ///
    /// Frees a frame that was previously allocated from the user frame pool.
    ///
    /// # Parameters
    ///
    /// - `uframe`: User frame to be freed.
    ///
    /// # Returns
    ///
    /// On success, empty is returned. On failure, an error is returned instead.
    ///
    pub fn free(&mut self, uframe: UserFrame) -> Result<(), Error> {
        frame::free(uframe.address())
    }
}
