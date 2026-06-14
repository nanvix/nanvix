// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Kernel frame type.
//!
//! A [`KernelFrame`] represents a single page-sized physical frame allocated for kernel use.
//! It provides `Deref`/`DerefMut` for direct kernel access (identity-mapped) and frees the
//! underlying frame on drop via the global frame allocator.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("kframe.spec.rs");
#[cfg(verus_keep_ghost)]
include!("kframe.proof.rs");

use crate::hal::mem::{
    Address,
    FrameAddress,
    PageAligned,
    PhysicalAddress,
};
use ::arch::mem;
use ::core::ops::{
    Deref,
    DerefMut,
};
use ::sys::error::Error;
use ::vstd::prelude::*;

//==================================================================================================
// Kernel Frame
//==================================================================================================

/// A type that represents a kernel frame.
#[derive(Debug)]
#[verus_verify]
pub struct KernelFrame {
    /// Frame address.
    base: FrameAddress,
}

#[cfg(verus_keep_ghost)]
verus! {

/// Abstract view of a [`KernelFrame`]: the base physical address of the frame it
/// owns. Lets allocator contracts name a returned kernel frame's address (e.g.
/// the contiguity guarantee of `alloc_many_kernel_frames`) without exposing any
/// storage detail.
impl View for KernelFrame {
    type V = int;

    closed spec fn view(&self) -> int {
        self.base@
    }
}

} // verus!

impl KernelFrame {
    ///
    /// # Description
    ///
    /// Instantiates a kernel frame.
    ///
    /// # Parameters
    ///
    /// - `base`: Frame address.
    ///
    /// # Returns
    ///
    /// Upon success, a kernel frame is returned. Upon failure, an error is returned instead.
    ///
    pub(super) fn new(base: FrameAddress) -> Result<Self, Error> { ... }

    ///
    /// # Description
    ///
    /// Returns the base address of the target kernel frame.
    ///
    /// # Returns
    ///
    /// The base address of the target kernel frame.
    ///
    pub fn base(&self) -> FrameAddress { ... }

    ///
    /// # Description
    ///
    /// Clears the target kernel frame.
    ///
    /// Uses the identity-map `memset` backend so that the write runs in the kernel address space.
    /// This avoids a page fault when the current CR3 points to a user page directory that lacks
    /// the PDE for this frame's physical address.
    ///
    pub fn clear(&mut self) -> Result<(), Error> { ... }
}

/// # Safety
///
/// `Deref` accesses the identity-mapped frame directly. The caller must ensure that the current
/// CR3 points to a page directory that has the relevant PDE for this frame's physical address
/// (i.e., the kernel page directory, or a user page directory into which the PDE has been
/// propagated via [`sync_kernel_pdes`](crate::mm::virt::sync_kernel_pdes)). For CR3-agnostic
/// writes, prefer [`KernelFrame::clear()`] which uses the identity-map `memset` backend.
impl Deref for KernelFrame {
    type Target = [u8];

    fn deref(&self) -> &Self::Target { ... }
}

/// # Safety
///
/// See [`Deref`] impl — the same CR3 invariant applies.
impl DerefMut for KernelFrame {
    fn deref_mut(&mut self) -> &mut Self::Target { ... }
}

impl Drop for KernelFrame {
    fn drop(&mut self) { ... }
}
