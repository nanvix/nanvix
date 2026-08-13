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

use crate::hal::mem::{
    Address,
    FrameAddress,
    PageAligned,
    PhysicalAddress,
};
use ::arch::mem;
#[cfg(verus_keep_ghost)]
use ::arch::mem::paging::PteWord;
use ::core::ops::{
    Deref,
    DerefMut,
};
use ::sys::error::Error;
use ::vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use ::vstd::raw_ptr::PointsTo;

include!("kframe.spec.rs");

//==================================================================================================
// Kernel Frame
//==================================================================================================

/// A type that represents a kernel frame.
#[verus_verify]
#[derive(Debug)]
pub struct KernelFrame {
    /// Frame address.
    base: FrameAddress,
}

#[verus_verify]
impl KernelFrame {
    /// Allocates a kernel frame for a paging structure and returns its raw entry permissions.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        with
            -> raw_permissions: Tracked<Map<nat, PointsTo<PteWord>>>,
        ensures
            match result {
                Ok(frame) => {
                    &&& raw_permissions.dom().len() == ::arch::mem::PAGE_TABLE_LENGTH
                    &&& forall|i: nat| raw_permissions.dom().contains(i)
                        <==> 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH
                    &&& forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
                        let permission = #[trigger] raw_permissions[i];
                        &&& permission.ptr()@.addr as int
                            == frame.base_address() + i * 4
                        &&& permission.is_uninit()
                    }
                },
                Err(_) => raw_permissions.dom().is_empty(),
            },
    )]
    pub(crate) fn allocate_page_table() -> Result<Self, Error> {
        let frame_address: FrameAddress = match super::frame::alloc() {
            Ok(frame_address) => frame_address,
            Err(error) => {
                proof_decl! {
                    let tracked raw_permissions:
                        Map<nat, PointsTo<PteWord>> = Map::tracked_empty();
                }
                return {
                    proof_with!(|= Tracked(raw_permissions));
                    Err(error)
                };
            },
        };
        match Self::new(frame_address) {
            Ok(frame) => {
                proof_decl! {
                    let tracked raw_permissions:
                        Map<nat, PointsTo<PteWord>> =
                            mint_kernel_frame_page_table_permissions(
                                frame.base_address(),
                            );
                }
                proof_with!(|= Tracked(raw_permissions));
                Ok(frame)
            },
            Err(error) => {
                if let Err(free_error) = super::frame::free(frame_address) {
                    warn!(
                        "failed to free frame after page-table allocation failure: {free_error:?}"
                    );
                }
                proof_decl! {
                    let tracked raw_permissions:
                        Map<nat, PointsTo<PteWord>> = Map::tracked_empty();
                }
                proof_with!(|= Tracked(raw_permissions));
                Err(error)
            },
        }
    }
}

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

impl Drop for KernelFrame {
    fn drop(&mut self) {
        if let Err(e) = super::frame::free(self.base) {
            error!("failed to free kernel frame: {:?}", e);
        }
    }
}
