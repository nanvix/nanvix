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
#[verus_verify(external_derive)]
pub struct UserFrame {
    /// Frame address.
    addr: FrameAddress,
}

#[cfg(verus_keep_ghost)]
verus! {

use crate::hal::mem::spec_page_size;
use crate::mm::phys::FrameAllocView;

/// Abstract view of a user frame: the physical address of the owned frame.
impl View for UserFrame {
    type V = int;

    closed spec fn view(&self) -> int {
        self.addr@
    }
}

/// Abstract view of the user page pool: the frame partition it draws from.
///
/// `Upool` is `external_body` (its real state is the global frame allocator), so its view
/// is uninterpreted — the trust obligation is tracked by the type being `external_body`.
impl View for Upool {
    type V = FrameAllocView;

    uninterp spec fn view(&self) -> FrameAllocView;
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
                    &&& crate::mm::phys::phys_view().frames.allocated_frames.contains(self@)
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
        ensures
            match result {
                Ok(count) => {
                    &&& crate::mm::phys::phys_view().frames.allocated_frames.contains(self@)
                    &&& count as int == crate::mm::phys::phys_view().frames.refcounts[self@]
                },
                Err(_) => !crate::mm::phys::phys_view().frames.allocated_frames.contains(self@),
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
            #[cfg(not(verus_keep_ghost))]
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
#[verus_verify(external_body)]
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
    // Dependency contract: opaque pool facade whose real backing store is the global frame
    // allocator. `external_body` (the `Upool` struct carries no spec-readable state) per
    // `verus-ai-logs/tcb-allowed.md`. The pool introduces no frames of its own; `wf()` is the
    // only fact its boot-time caller needs before handing the pool to `PhysMemoryManager::init`.
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
    // Dependency contract: delegates to the global frame allocator (`frame::alloc`). Modeled
    // as a watermark-agnostic single-frame allocation over the pool's frame partition. Marked
    // `external_body` until the `frame` free-function layer is verified.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self)@.wf(),
        ensures
            final(self)@.wf(),
            match result {
                Ok(uf) => {
                    &&& old(self)@.free_frames.contains(uf@)
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
