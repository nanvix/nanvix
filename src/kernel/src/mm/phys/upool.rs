// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("upool.spec.rs");
#[cfg(verus_keep_ghost)]
include!("upool.proof.rs");

use crate::{
    hal::mem::FrameAddress,
    mm::phys::frame,
};
use ::core::mem::ManuallyDrop;
use ::sys::error::Error;
use ::vstd::prelude::*;

//==================================================================================================
// User Frame
//==================================================================================================

///
/// # Description
///
/// A type that represents a user frame.
///
#[derive(Debug)]
#[verus_verify]
pub struct UserFrame {
    /// Frame address.
    addr: FrameAddress,
}

#[cfg(verus_keep_ghost)]
verus! {

/// Abstract view of a [`UserFrame`]: the physical address of the frame it owns.
///
/// Lets allocator contracts name a returned user frame's address (e.g. "the
/// returned frame is now allocated") without exposing any storage detail.
impl View for UserFrame {
    type V = int;

    closed spec fn view(&self) -> int {
        self.addr@
    }
}

} // verus!

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

    ///
    /// # Description
    ///
    /// Adds a new reference to the underlying physical frame and returns a fresh
    /// [`UserFrame`] handle that owns that reference. The two handles share the
    /// same physical frame, and the frame is only reclaimed once both handles are
    /// dropped.
    ///
    /// This is the building block for copy-on-write sharing: the parent retains
    /// its handle, the child receives the returned handle.
    ///
    /// # Returns
    ///
    /// On success, a new [`UserFrame`] that aliases the same physical frame as
    /// `self`. On failure, an error is returned.
    ///
    pub fn share(&self) -> Result<UserFrame, Error> {
        frame::share(self.addr)?;
        Ok(Self { addr: self.addr })
    }

    ///
    /// # Description
    ///
    /// Returns the current reference count of the underlying physical frame.
    ///
    /// # Returns
    ///
    /// Upon success, the current reference count of the underlying physical frame is returned.
    /// Upon failure, an error is returned instead.
    ///
    pub fn refcount(&self) -> Result<u8, Error> {
        frame::refcount(self.addr)
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
/// so user-frame allocation has its own entry point ([`Upool::alloc`] returning [`UserFrame`]).
///
#[derive(Debug)]
#[verus_verify]
pub struct Upool {
    /// Private field prevents external construction.
    _private: (),
}

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
    pub(super) fn new() -> Self {
        Self { _private: () }
    }

    ///
    /// # Description
    ///
    /// Allocates a single user frame from the user frame pool.
    ///
    /// # Returns
    ///
    /// Upon success, a user frame is returned. Upon failure, an error is returned instead.
    ///
    pub fn alloc(&mut self) -> Result<UserFrame, Error> {
        let addr: FrameAddress = frame::alloc()?;
        Ok(UserFrame::new(addr))
    }
}
