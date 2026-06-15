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

//==================================================================================================
// Kernel Frame
//==================================================================================================

/// A type that represents a kernel frame.
#[derive(Debug)]
#[verus_verify(external_derive)]
pub struct KernelFrame {
    /// Frame address.
    base: FrameAddress,
}

// Dependency contract for the manager layer: wrapping a `FrameAddress` produces a handle whose
// abstract address is the same physical frame.
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
    // Wrapping a frame produces a handle whose abstract address is the same physical frame
    // (`kf@ == base@`) and which is well-formed (`kf.inv()`, page alignment carried from
    // `base.inv()`). The identity-mapping side effect is delegated to [`KernelFrame::map_frame`],
    // an exec-only helper outside Verus' purview (it materializes a page-table entry through the
    // not-yet-verified `mm::virt` layer, whose global `identity_map_view().inv()` precondition
    // cannot be discharged from within `mm::phys`). Verus verifies only the pure wrap here.
    #[verus_spec(result =>
        requires
            base.inv(),
        ensures
            match result {
                Ok(kf) => {
                    &&& kf@ == base@
                    &&& kf.inv()
                },
                Err(_) => true,
            },
    )]
    pub(super) fn new(base: FrameAddress) -> Result<Self, Error> {
        // Ensure the frame is identity-mapped in the kernel address space so that
        // Deref/DerefMut can safely access it. This lazily installs a page
        // table entry if needed (page tables come from a BSS pool, so no recursive frame
        // allocation occurs).
        Self::map_frame(base)?;

        Ok(Self { base })
    }
}

// Exec-only helper: installs the identity mapping for `base` via `mm::virt::identity_map_page`.
// VERUS REWRITE: extracted from the body of `KernelFrame::new` so the pure frame-wrap can be
// machine-verified while the cross-module page-table side effect is isolated behind a single,
// empty trusted contract (`assume_specification[ KernelFrame::map_frame ]` in kframe.spec.rs,
// ledgered in verus-ai-logs/tcb-allowed.md). Inlining the side effect back into the verified
// `new` is impossible: (1) `identity_map_page` requires `identity_map_view().inv()`, an
// `uninterp spec fn` in the PRIVATE `mod identity_map` that is not re-exported and therefore
// cannot even be NAMED from `mm::phys`; (2) `PageAligned::from_raw_value` is external and would
// itself need an `assume_specification`; (3) the `error!` logging macros fail to verify
// ("Unsupported constant type") and would require cfg-gated exec. Option A thus multiplies
// trusted surface and still cannot discharge the precondition; this extraction trusts strictly
// less. Kept outside Verus (no `#[verus_verify]`); carries no `external_body`.
impl KernelFrame {
    fn map_frame(base: FrameAddress) -> Result<(), Error> {
        let phys_addr: PageAligned<PhysicalAddress> =
            PageAligned::from_raw_value(base.into_raw_value()).map_err(|e| {
                error!("frame base is not page-aligned: {e:?}");
                e
            })?;
        crate::mm::virt::identity_map_page(phys_addr).map_err(|e| {
            error!("failed to identity-map frame: {:?}", e);
            e
        })
    }
}

#[verus_verify]
impl KernelFrame {

    ///
    /// # Description
    ///
    /// Returns the base address of the target kernel frame.
    ///
    /// # Returns
    ///
    /// The base address of the target kernel frame.
    ///
    // Pure accessor: returns the owned frame's physical address unchanged. `result@ == self@`
    // gives `kpage` the exact address; `result.inv()` (page alignment carried from `self.inv()`)
    // lets `into_page_address` and the `KernelStack` arithmetic behind it proceed.
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result@ == self@,
            result.inv(),
    )]
    pub fn base(&self) -> FrameAddress {
        self.base
    }
}

impl KernelFrame {

    ///
    /// # Description
    ///
    /// Clears the target kernel frame.
    ///
    /// Uses the identity-map `memset` backend so that the write runs in the kernel address space.
    /// This avoids a page fault when the current CR3 points to a user page directory that lacks
    /// the PDE for this frame's physical address.
    ///
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
    // Releasing the handle returns its physical frame to the global frame allocator via
    // `super::frame::free`, which is best-effort (`ensures true`) and `opens_invariants none`/
    // `no_unwind`, so `drop` makes no abstract postcondition. Callers (the manager error path,
    // `KernelStack::drop`) rely on this being the sole, complete deallocation step. Mirror of
    // `UserFrame::drop`.
    #[verus_spec(
        opens_invariants none
        no_unwind
    )]
    fn drop(&mut self) {
        if let Err(e) = super::frame::free(self.base) {
            #[cfg(not(verus_keep_ghost))]
            error!("failed to free kernel frame: {:?}", e);
        }
    }
}
