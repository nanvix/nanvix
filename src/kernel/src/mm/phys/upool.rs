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

//==================================================================================================
// User Frame
//==================================================================================================

///
/// # Description
///
/// A type that represents a user frame.
///
#[verus_verify(external_derive)]
#[derive(Debug)]
pub struct UserFrame {
    /// Frame address.
    addr: FrameAddress,
}

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
        requires addr.inv(),
        ensures
            ret.inv(),
            ret@ == addr@,
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
        ensures ret@ == self@,
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
        ensures ret@ == self@,
    )]
    pub fn leak(self) -> FrameAddress {
        let this: ManuallyDrop<Self> = ManuallyDrop::new(self);
        this.addr
    }
}

// NOTE: Drop must use verus!{} syntax because Verus requires
// `opens_invariants none no_unwind` on Drop impls, which the
// attribute-based syntax does not support.
verus! {
impl Drop for UserFrame {
    fn drop(&mut self)
        opens_invariants none
        no_unwind
    {
        // VERUS REWRITE: renamed e -> _e to suppress unused-variable warning
        // when the error! logging macro is cfg-gated out under Verus.
        if let Err(_e) = frame::free(self.addr) {
            #[cfg(not(verus_keep_ghost))]
            error!("failed to free user frame: {:?}", _e);
        }
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
#[verus_verify(external_derive)]
#[derive(Debug)]
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
    #[verus_spec]
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
    #[verus_spec(ret =>
        ensures
            match ret {
                Ok(frame) => frame.inv(),
                Err(_) => true,
            },
    )]
    pub fn alloc(&mut self) -> Result<UserFrame, Error> {
        let addr: FrameAddress = frame::alloc()?;
        Ok(UserFrame::new(addr))
    }
}
