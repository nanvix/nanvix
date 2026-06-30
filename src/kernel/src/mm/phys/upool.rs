// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("upool.spec.rs");

use crate::{
    hal::mem::FrameAddress,
    mm::phys::frame,
};
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
    #[verus_spec(result =>
        requires
            addr.inv(),
        ensures
            result@ == addr@,
            result.inv(),
    )]
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
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result@ == self@,
            result.inv(),
    )]
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
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result@ == self@,
            result.inv(),
    )]
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
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            match result {
                Ok(uf) => {
                    &&& uf@ == self@
                    &&& uf.inv()
                    &&& crate::mm::phys::phys_view().frames.is_allocated(self@)
                },
                Err(_) => {
                    ||| !crate::mm::phys::phys_view().frames.is_allocated(self@)
                    ||| crate::mm::phys::phys_view().frames.refcounts[self@] >= 255
                },
            },
    )]
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
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            match result {
                Ok(count) => {
                    &&& crate::mm::phys::phys_view().frames.is_allocated(self@)
                    &&& count as int == crate::mm::phys::phys_view().frames.refcounts[self@]
                },
                Err(_) => !crate::mm::phys::phys_view().frames.is_allocated(self@),
            },
    )]
    pub fn refcount(&self) -> Result<u8, Error> {
        frame::refcount(self.addr)
    }
}

impl Drop for UserFrame {
    #[verus_spec(
        opens_invariants none
        no_unwind
    )]
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
    /// Opaque pool facade backed by the global frame allocator.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        ensures
            result@.wf(),
    )]
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
    /// Delegates to the global frame allocator and models one user-frame allocation.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self)@.wf(),
        ensures
            final(self)@.wf(),
            match result {
                Ok(uf) => {
                    &&& old(self)@.is_free(uf@)
                    &&& final(self)@ == old(self)@.alloc_one(uf@)
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& old(self)@.free_count() == 0
                },
            },
    )]
    pub fn alloc(&mut self) -> Result<UserFrame, Error> {
        let addr: FrameAddress = frame::alloc()?;
        Ok(UserFrame::new(addr))
    }
}
