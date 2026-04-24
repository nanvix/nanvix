// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Frame allocator — module-level singleton.
//!
//! The frame allocator is backed by a [`SparseBitmap`] and exposed as free functions over a
//! singleton so every in-kernel caller (upool, kpool, anything else that needs a raw frame) goes
//! through the same state. No struct-valued handle is passed around.
//!
//! Access to the frame allocator is synchronized externally and performed by a single thread, so
//! the backing bitmap uses non-atomic operations.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::{
    FrameAddress,
    PageAligned,
    PhysicalAddress,
    TruncatedMemoryRegion,
};
use ::arch::mem::{
    self,
    paging::FrameNumber,
};
use ::config::constants;
use ::core::{
    hint::unlikely,
    mem::MaybeUninit,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
use ::sparse_bitmap::SparseBitmap;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Address,
};
use ::vstd::prelude::*;

#[cfg(verus_keep_ghost)]
include!("frame.spec.rs");

#[cfg(verus_keep_ghost)]
include!("frame.proof.rs");

//==================================================================================================
// Inner
//==================================================================================================

/// Private state of the frame allocator singleton.
#[verus_verify]
struct Inner {
    /// A sparse bitmap that keeps track of free/used frames.
    bitmap: SparseBitmap,
}

#[verus_verify]
impl Inner {
    ///
    /// # Description
    ///
    /// Allocates a frame.
    ///
    /// # Returns
    ///
    /// Upon success, the address of the allocated frame is returned. Upon failure, an error is
    /// returned instead.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            self.inv(),
            match result {
                Ok(frame) => {
                    &&& frame.inv()
                    &&& old(self)@.free_frames.contains(frame@)
                    &&& self@ == UpoolView {
                        allocated_frames: old(self)@.allocated_frames.insert(frame@),
                        free_frames: old(self)@.free_frames.remove(frame@),
                    }
                },
                Err(_) => {
                    &&& self@ == old(self)@
                    &&& old(self)@.free_frames.is_empty()
                }
            },
    )]
    fn alloc(&mut self) -> Result<FrameAddress, Error> { ... }

    ///
    /// # Description
    ///
    /// Frees a frame that was previously allocated.
    ///
    /// # Parameters
    ///
    /// - `frame`: Address of the frame to free.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self).inv(),
            frame.inv(),
        ensures
            self.inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.allocated_frames.contains(frame@)
                    &&& self@ == UpoolView {
                        allocated_frames: old(self)@.allocated_frames.remove(frame@),
                        free_frames: old(self)@.free_frames.insert(frame@),
                    }
                },
                Err(_) => {
                    &&& self@ == old(self)@
                    &&& !old(self)@.allocated_frames.contains(frame@)
                }
            },
    )]
    fn free(&mut self, frame: FrameAddress) -> Result<(), Error> { ... }

    ///
    /// # Description
    ///
    /// Books a frame so that it will not be handed out by [`alloc`].
    ///
    /// # Parameters
    ///
    /// - `phys_addr`: Physical address of the frame to book.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self).inv(),
            phys_addr.inv(),
        ensures
            self.inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.free_frames.contains(phys_addr@)
                    &&& self@ == UpoolView {
                        allocated_frames: old(self)@.allocated_frames.insert(phys_addr@),
                        free_frames: old(self)@.free_frames.remove(phys_addr@),
                    }
                },
                Err(_) => {
                    &&& self@ == old(self)@
                    &&& !old(self)@.free_frames.contains(phys_addr@)
                }
            },
    )]
    fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> { ... }

    ///
    /// # Description
    ///
    /// Allocates all frames in the given region.
    ///
    /// # Parameters
    ///
    /// - `region`: Physical memory region whose frames should be booked.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self).inv(),
            region.inv(),
        ensures
            self.inv(),
            ({
                let start_frame_number = region@.start / spec_page_size();
                let end_frame_number = (region@.start + region@.size) / spec_page_size();
                let frame_numbers = vstd::set_lib::set_int_range(start_frame_number, end_frame_number);
                let frames = frame_numbers.map(|i: int| i * spec_page_size());
                match result {
                    Ok(()) => {
                        &&& frames.subset_of(old(self)@.free_frames)
                        &&& self@ == UpoolView {
                            allocated_frames: old(self)@.allocated_frames.union(frames),
                            free_frames: old(self)@.free_frames.difference(frames),
                        }
                    },
                    Err(_) => {
                        &&& self@ == old(self)@
                        &&& !frames.subset_of(old(self)@.free_frames)
                    },
                }
            }),
    )]
    fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error> { ... }
}

//==================================================================================================
// Constants
//==================================================================================================

// Use relaxed ordering for all atomic operations to mitigate synchronization overhead. It is safe
// to use this ordering semantics because Nanvix is a single-core system, and the kernel runs with
// interrupts disabled.
const ORDER: Ordering = Ordering::Relaxed;

//==================================================================================================
// Singleton
//==================================================================================================

/// Module-level singleton storage.
static mut INSTANCE: MaybeUninit<Inner> = MaybeUninit::uninit();

/// Whether the frame allocator has been initialized.
static INSTANCE_INIT: AtomicBool = AtomicBool::new(false);

/// Returns a mutable reference to the initialized singleton.
fn instance() -> &'static mut Inner { ... }

//==================================================================================================
// Public Free Functions
//==================================================================================================

/// Initialize the frame allocator singleton.
///
/// # Safety
///
/// Must be called exactly once during boot, before any other function
/// in this module.
pub(super) unsafe fn init(bitmap: SparseBitmap) -> Result<(), Error> { ... }

/// Allocate a frame.
#[verus_verify(external_body)]
#[verus_spec(result =>
    ensures
        match result {
            Ok(frame) => frame.inv(),
            Err(_) => true,
        },
)]
pub(super) fn alloc() -> Result<FrameAddress, Error> { ... }

// NOTE: free uses verus! syntax because Drop::drop requires `no_unwind`,
// and the attribute-based syntax does not support `no_unwind`.
verus! {
/// Free a frame previously returned by [`alloc`].
#[verifier::external_body]
pub(super) fn free(frame: FrameAddress) -> (result: Result<(), Error>)
    opens_invariants none
    no_unwind
{ ... }
}

/// Reserve a frame so [`alloc`] will skip it.
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> { ... }

/// Book every frame in the given physical memory region.
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error> { ... }
