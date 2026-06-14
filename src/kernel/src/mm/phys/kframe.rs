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
#[cfg(verus_keep_ghost)]
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

#[verus_verify]
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
    #[verus_spec(result =>
        ensures
            // Address identity: on success the handle owns exactly the input
            // address, so callers can transfer the allocator facts established
            // for `base` (membership, alignment) onto the returned handle. On
            // failure no handle exists; `base` is `Copy`, so nothing is consumed
            // and the caller remains free to release the raw frame.
            match result {
                Ok(frame) => frame@ == base@,
                Err(_) => true,
            },
            phys_view().inv(),
    )]
    pub(super) fn new(base: FrameAddress) -> Result<Self, Error> {
        // Ensure the frame is identity-mapped in the kernel address space so that
        // Deref/DerefMut can safely access it. This lazily installs a page
        // table entry if needed (page tables come from a BSS pool, so no recursive frame
        // allocation occurs).
        let phys_addr: PageAligned<PhysicalAddress> =
            PageAligned::from_raw_value(base.into_raw_value()).map_err(|e| {
                error!("frame base is not page-aligned: {e:?}");
                e
            })?;
        crate::mm::virt::identity_map_page(phys_addr).map_err(|e| {
            error!("failed to identity-map frame: {:?}", e);
            e
        })?;

        Ok(Self { base })
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the target kernel frame.
    ///
    /// # Returns
    ///
    /// The base address of the target kernel frame.
    ///
    #[verus_spec(result =>
        ensures
            // Pure read: the returned address is the handle's abstract value.
            result@ == self@,
    )]
    pub fn base(&self) -> FrameAddress {
        self.base
    }

    ///
    /// # Description
    ///
    /// Clears the target kernel frame.
    ///
    /// Uses the identity-map `memset` backend so that the write runs in the kernel address space.
    /// This avoids a page fault when the current CR3 points to a user page directory that lacks
    /// the PDE for this frame's physical address.
    ///
    // Trusted (TCB, out of verification scope): materializes a `*mut u8` from the
    // frame's raw address (`usize as *mut u8`) and writes through the identity-map
    // `memset` backend -- a raw-memory operation Verus cannot model. Listed in
    // `verus-ai-logs/tcb-allowed.md`.
    #[verus_verify(external_body)]
    pub fn clear(&mut self) -> Result<(), Error> {
        let base: *mut u8 = self.base.into_raw_value() as *mut u8;
        crate::mm::virt::memset(base, 0, mem::PAGE_SIZE).map_err(|e| {
            error!("memset failed: {:?}", e);
            e
        })
    }
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

    fn deref(&self) -> &Self::Target {
        let ptr: usize = self.base.into_raw_value();
        unsafe { core::slice::from_raw_parts(ptr as *const u8, mem::PAGE_SIZE) }
    }
}

/// # Safety
///
/// See [`Deref`] impl — the same CR3 invariant applies.
impl DerefMut for KernelFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr: usize = self.base.into_raw_value();
        unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, mem::PAGE_SIZE) }
    }
}

#[verus_verify]
impl Drop for KernelFrame {
    #[verus_spec(
        ensures
            // Releasing the frame preserves the subsystem invariant (the last
            // reference returns the frame to the free pool). The precise refcount
            // transition is not expressible: `phys_view()` is a single fixed value
            // with no `old(phys_view())` to compare against. Errors are logged, not
            // propagated, so `drop` cannot unwind.
            phys_view().inv(),
        opens_invariants none
        no_unwind
    )]
    fn drop(&mut self) {
        if let Err(e) = super::frame::free(self.base) {
            error!("failed to free kernel frame: {:?}", e);
        }
    }
}
