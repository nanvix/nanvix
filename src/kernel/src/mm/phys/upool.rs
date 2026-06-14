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

#[verus_verify]
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
    #[verus_spec(ret =>
        requires
            addr.inv(),
        ensures
            // `new` merely names an existing frame: the handle's address is `addr`
            // and it inherits `addr`'s page-alignment. No allocation and no
            // refcount change occur (ownership semantics come from `Drop`/`leak`),
            // so `new` says nothing about `phys_view()`.
            ret@ == addr@,
            ret.inv(),
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
    #[verus_spec(ret =>
        ensures
            // Pure getter: returns exactly the address the handle owns.
            ret@ == self@,
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
    #[verus_spec(ret =>
        ensures
            // Consumes the handle and returns its address without releasing the
            // reference: `Drop` is suppressed (via `ManuallyDrop`), so the frame
            // stays allocated and `phys_view()` is unchanged.
            ret@ == self@,
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
    #[verus_spec(child =>
        requires
            self.inv(),
            phys_view().initialized,
            phys_view().inv(),
        ensures
            phys_view().inv(),
            phys_view().initialized,
            // On success: a fresh handle aliasing the same physical frame (equal
            // view), well-formed, and the frame is still allocated (it has gained
            // a reference). On failure: no reference acquired; `self` is untouched.
            match child {
                Ok(handle) => {
                    &&& handle@ == self@
                    &&& handle.inv()
                    &&& phys_view().frames.allocated_frames.contains(handle@)
                },
                Err(_) => true,
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
            phys_view().initialized,
            phys_view().inv(),
        ensures
            phys_view().inv(),
            phys_view().initialized,
            // Pure query of the underlying frame's reference count: neither
            // consumes `self` nor changes the count. On success the frame is
            // allocated and the returned count equals its refcount; on failure the
            // frame is not allocated.
            match result {
                Ok(count) => {
                    &&& phys_view().frames.allocated_frames.contains(self@)
                    &&& phys_view().frames.refcounts.contains_key(self@)
                    &&& count as int == phys_view().frames.refcounts[self@]
                },
                Err(_) => !phys_view().frames.allocated_frames.contains(self@),
            },
    )]
    pub fn refcount(&self) -> Result<u8, Error> {
        frame::refcount(self.addr)
    }
}

#[verus_verify]
impl Drop for UserFrame {
    #[verus_verify(external_body)]
    #[verus_spec(
        ensures
            // Releasing a reference preserves the subsystem invariant (the last
            // reference returns the frame to the free pool). The precise refcount
            // transition is not expressible: `phys_view()` is a single fixed value
            // with no `old(phys_view())` to compare against. Errors are logged, not
            // propagated, so `drop` cannot unwind.
            phys_view().inv(),
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
#[verus_verify]
pub struct Upool {
    /// Private field prevents external construction.
    _private: (),
}

#[verus_verify]
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
    #[verus_verify]
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
    #[verus_spec(result =>
        requires
            phys_view().initialized,
            phys_view().inv(),
        ensures
            phys_view().inv(),
            phys_view().initialized,
            // On success: a handle owning a freshly allocated, page-aligned frame
            // whose address is now reserved (`allocated_frames`) with a single
            // reference. On failure: nothing is reported about the absent frame.
            match result {
                Ok(uf) => {
                    &&& uf.inv()
                    &&& phys_view().frames.allocated_frames.contains(uf@)
                    &&& phys_view().frames.refcounts.contains_key(uf@)
                    &&& phys_view().frames.refcounts[uf@] == 1
                },
                Err(_) => true,
            },
    )]
    pub fn alloc(&mut self) -> Result<UserFrame, Error> {
        let addr: FrameAddress = frame::alloc()?;
        Ok(UserFrame::new(addr))
    }
}
