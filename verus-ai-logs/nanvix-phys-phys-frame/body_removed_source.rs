// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Frame allocator — module-level singleton.
//!
//! The frame allocator is backed by a [`Bitmap`] and exposed as free functions over a
//! singleton so every in-kernel caller goes through the same state. No struct-valued handle is
//! passed around.
//!
//! Access to the frame allocator is synchronized externally and performed by a single thread, so
//! the backing bitmap uses non-atomic operations.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    mem::{
        FrameAddress,
        PageAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
    },
    platform::NFRAMES,
};
use ::arch::mem::{
    self,
    paging::FrameNumber,
};
use ::bitmap::Bitmap;
use ::config::constants;
use ::core::{
    hint::unlikely,
    mem::MaybeUninit,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
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

/// BSS-backed per-frame reference count storage. Indexed by frame number.
///
/// Sits in BSS rather than on the kernel heap because the slab allocator caps single
/// allocations at a few hundred bytes, but a full refcount table for the configured
/// memory size is much larger.
///
/// # Size impact
///
/// This array is unconditionally reserved in BSS and scales linearly with the
/// configured machine memory size: `MEMORY_SIZE / FRAME_SIZE * size_of::<u8>()`
/// bytes (e.g. 256 KiB for a 1 GiB configuration). A `u8` is sufficient because at
/// most [`config::kernel::MAX_PROCESSES`] (≤ 255) processes can simultaneously share a
/// frame, so the refcount of any frame is bounded by 255.
///
/// # Safety
///
/// Accessed only through `Inner::refcount`, which is set up at boot from this storage
/// and never aliased. The kernel is single-threaded and runs with interrupts disabled,
/// so non-atomic access is sound.
static mut REFCOUNT_STORAGE: [u8; NFRAMES] = [0; NFRAMES];

/// Private state of the frame allocator singleton.
#[verus_verify]
struct Inner {
    /// A bitmap that keeps track of free/used frames.
    bitmap: Bitmap,
    /// Per-frame reference count. Indexed by frame number.
    ///
    /// Invariants:
    ///
    /// - `refcount.len() >= bitmap.number_of_bits()`.
    /// - `refcount[i] >= 1` iff bit `i` is set in `bitmap` (for `i < bitmap.number_of_bits()`).
    /// - `refcount[i] == 0` iff bit `i` is clear in `bitmap` (for `i < bitmap.number_of_bits()`).
    ///
    /// A refcount greater than one means that the frame is shared between multiple
    /// owners (e.g. parent and child after [`share`]). The frame is reclaimed (bitmap
    /// bit cleared) only when the refcount reaches zero.
    ///
    /// The element type is `u8`: the kernel caps the number of live processes at
    /// [`config::kernel::MAX_PROCESSES`] (≤ 255), so a frame can be shared by at most 255
    /// owners and the count always fits in a byte.
    refcount: &'static mut [u8],
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
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(frame) => {
                    &&& frame.inv()
                    &&& old(self)@.free_frames.contains(frame@)
                    &&& final(self)@ == FrameAllocView {
                        allocated_frames: old(self)@.allocated_frames.insert(frame@),
                        free_frames: old(self)@.free_frames.remove(frame@),
                        refcounts: old(self)@.refcounts.insert(frame@, 1int),
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& old(self)@.free_frames.is_empty()
                }
            },
    )]
    fn alloc(&mut self) -> Result<FrameAddress, Error> { ... }

    ///
    /// # Description
    ///
    /// Allocates `count` physically contiguous frames.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of contiguous frames to allocate.
    ///
    /// # Returns
    ///
    /// Upon success, the base `FrameAddress` of the contiguous range is returned. Upon failure,
    /// an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            count > 0,
        ensures
            final(self).inv(),
            match result {
                Ok(base) => {
                    &&& base.inv()
                    &&& ({
                        let frames = Set::new(|addr: int|
                            exists|i: int| 0 <= i < count && addr == #[trigger] (base@ + i * spec_page_size())
                        );
                        &&& frames.subset_of(old(self)@.free_frames)
                        &&& final(self)@ == FrameAllocView {
                            allocated_frames: old(self)@.allocated_frames.union(frames),
                            free_frames: old(self)@.free_frames.difference(frames),
                            refcounts: old(self)@.refcounts.union_prefer_right(
                                Map::new(|addr: int| frames.contains(addr), |addr: int| 1int)
                            ),
                        }
                    })
                },
                Err(_) => {
                    final(self)@ == old(self)@
                }
            },
    )]
    fn alloc_contiguous(&mut self, count: usize) -> Result<FrameAddress, Error> { ... }

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
    #[verus_spec(result =>
        requires
            old(self).inv(),
            frame.inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.allocated_frames.contains(frame@)
                    &&& old(self)@.refcounts.contains_key(frame@)
                    &&& old(self)@.refcounts[frame@] > 0
                    &&& if old(self)@.refcounts[frame@] == 1 {
                        // Last reference: release frame
                        final(self)@ == FrameAllocView {
                            allocated_frames: old(self)@.allocated_frames.remove(frame@),
                            free_frames: old(self)@.free_frames.insert(frame@),
                            refcounts: old(self)@.refcounts.remove(frame@),
                        }
                    } else {
                        // Still shared: decrement refcount
                        final(self)@ == FrameAllocView {
                            allocated_frames: old(self)@.allocated_frames,
                            free_frames: old(self)@.free_frames,
                            refcounts: old(self)@.refcounts.insert(
                                frame@, old(self)@.refcounts[frame@] - 1
                            ),
                        }
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& !old(self)@.allocated_frames.contains(frame@)
                }
            },
    )]
    fn free(&mut self, frame: FrameAddress) -> Result<(), Error> { ... }

    ///
    /// # Description
    ///
    /// Adds a new reference to a frame that has already been allocated.
    ///
    /// This is used to implement page sharing (e.g. for copy-on-write). The matching
    /// number of [`free`] calls must be issued to actually release the frame back to
    /// the bitmap.
    ///
    /// # Parameters
    ///
    /// - `frame`: Address of the frame to share.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            frame.inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.allocated_frames.contains(frame@)
                    &&& old(self)@.refcounts.contains_key(frame@)
                    &&& final(self)@ == FrameAllocView {
                        allocated_frames: old(self)@.allocated_frames,
                        free_frames: old(self)@.free_frames,
                        refcounts: old(self)@.refcounts.insert(
                            frame@, old(self)@.refcounts[frame@] + 1
                        ),
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& (
                        !old(self)@.allocated_frames.contains(frame@)
                        || (old(self)@.refcounts.contains_key(frame@)
                            && old(self)@.refcounts[frame@] >= 255)
                    )
                }
            },
    )]
    fn share(&mut self, frame: FrameAddress) -> Result<(), Error> { ... }

    ///
    /// # Description
    ///
    /// Returns the current reference count of an already-allocated frame.
    ///
    /// # Parameters
    ///
    /// - `frame`: Address of the frame to query.
    ///
    /// # Returns
    ///
    /// Upon success, the current reference count is returned. Upon failure, an error is
    /// returned instead (out-of-bounds address, or the frame is not currently allocated).
    ///
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            self.inv(),
            match result {
                Ok(count) => {
                    &&& self@.allocated_frames.contains(frame@)
                    &&& self@.refcounts.contains_key(frame@)
                    &&& count as int == self@.refcounts[frame@]
                },
                Err(_) => {
                    !self@.allocated_frames.contains(frame@)
                }
            },
    )]
    fn refcount(&self, frame: FrameAddress) -> Result<u8, Error> { ... }

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
    #[verus_spec(result =>
        requires
            old(self).inv(),
            phys_addr.inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.free_frames.contains(phys_addr@)
                    &&& final(self)@ == FrameAllocView {
                        allocated_frames: old(self)@.allocated_frames.insert(phys_addr@),
                        free_frames: old(self)@.free_frames.remove(phys_addr@),
                        refcounts: old(self)@.refcounts.insert(phys_addr@, 1int),
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& !old(self)@.free_frames.contains(phys_addr@)
                }
            },
    )]
    fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> { ... }

    ///
    /// # Description
    ///
    /// Checks whether the frame allocator tracks the frame at the given physical address.
    ///
    /// # Returns
    ///
    /// `true` if the frame allocator tracks the frame at `phys_addr`, `false` otherwise.
    ///
    #[verus_spec(ret =>
        requires
            self.inv(),
            phys_addr.inv(),
        ensures
            self.inv(),
            ret <==> (
                self@.allocated_frames.contains(phys_addr@)
                || self@.free_frames.contains(phys_addr@)
            ),
    )]
    fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool { ... }

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
    #[verus_spec(result =>
        requires
            old(self).inv(),
            region.inv(),
        ensures
            final(self).inv(),
            ({
                let start_frame_number = region@.start / spec_page_size();
                let end_frame_number = (region@.start + region@.size) / spec_page_size();
                let frame_numbers = vstd::set_lib::set_int_range(start_frame_number, end_frame_number);
                let frames = frame_numbers.map(|i: int| i * spec_page_size());
                match result {
                    Ok(()) => {
                        &&& frames.subset_of(old(self)@.free_frames)
                        &&& final(self)@ == FrameAllocView {
                            allocated_frames: old(self)@.allocated_frames.union(frames),
                            free_frames: old(self)@.free_frames.difference(frames),
                            refcounts: old(self)@.refcounts.union_prefer_right(
                                Map::new(|addr: int| frames.contains(addr), |addr: int| 1int)
                            ),
                        }
                    },
                    Err(_) => {
                        &&& final(self)@ == old(self)@
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
//
// Trust boundary: materializes a `&'static mut Inner` from the module-level `static mut`
// singleton storage (`INSTANCE: MaybeUninit<Inner>`), guarded by `INSTANCE_INIT`. This is a
// raw-memory operation over externally-owned storage that Verus cannot model without a
// `PointsTo` for the `static mut` (mirrors the `bump_allocator` materialization). The `ensures`
// pins the abstract state of the singleton to the global subsystem view (`phys_view().frames`)
// and records that the allocator is initialized — the §8 ghost-token attachment realized here.
#[verus_verify(external_body)]
#[verus_spec(r =>
    ensures
        (*r).inv(),
        (*r)@ == crate::mm::phys::phys_view().frames,
        crate::mm::phys::phys_view().initialized,
)]
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
//
// Skip/exclude target (see `verus-ai-logs/tcb-allowed.md`): initializes the `static mut`
// singleton and the BSS-backed refcount storage. `external_body` because it materializes
// `&'static mut REFCOUNT_STORAGE` and writes the `MaybeUninit` singleton — raw-memory ops Verus
// cannot verify. Callers rely on it establishing `phys_view().initialized` (via
// `lemma_frame_initialized`) before any other free function runs.
#[verus_verify(external_body)]
pub(super) unsafe fn init(bitmap: Bitmap) -> Result<(), Error> { ... }

/// Allocate a frame.
// Dependency contract for the manager layer: thin singleton wrapper around `Inner::alloc`.
// `external_body` until the `frame` free-function layer is verified; the manager bridges the
// returned address into its own abstract frame partition via a proof lemma.
#[verus_spec(result =>
    ensures
        match result {
            Ok(frame) => {
                &&& frame.inv()
                &&& crate::mm::phys::phys_view().frames.allocated_frames.contains(frame@)
            },
            Err(_) => crate::mm::phys::phys_view().frames.free_frames.is_empty(),
        },
)]
pub(super) fn alloc() -> Result<FrameAddress, Error> { ... }

/// # Description
///
/// Allocates `count` physically contiguous frames.
///
/// # Returns
///
/// Returns the base `FrameAddress` of the contiguous range.
///
// Dependency contract: thin singleton wrapper around `Inner::alloc_contiguous`. The base
// address is page-aligned on success. The address-space range bound (`base@ + count*PS <=
// usize::MAX`) is the fact the manager's per-frame index arithmetic relies upon; it follows
// from `Inner::alloc_contiguous`'s frame-set postcondition plus the allocator invariant
// (bridged in the proving phase).
#[verus_spec(result =>
    requires
        count > 0,
    ensures
        match result {
            Ok(base) => {
                &&& base.inv()
                &&& base@ + (count as int) * spec_page_size() <= usize::MAX as int
            },
            Err(_) => true,
        },
)]
pub(super) fn alloc_contiguous(count: usize) -> Result<FrameAddress, Error> { ... }

///
/// # Description
///
/// Returns the number of free frames in the system.
///
/// # Returns
///
/// The number of free frames in the system.
///
// Dependency contract: reports the size of the free partition of the global frame allocator.
// The bitmap-level count (`number_of_bits - usage`) equals the abstract `free_count()`
// (`free_frames.len()`); this is bridged in the proving phase.
#[verus_spec(result =>
    ensures
        result as nat == crate::mm::phys::phys_view().frames.free_count(),
)]
pub(super) fn free_count() -> usize { ... }

/// Free a frame previously returned by [`alloc`].
// Dependency contract: best-effort release of a frame. Callers (the manager's error-cleanup
// paths and the `Drop` impls) ignore the outcome, so no precondition is imposed and no abstract
// postcondition is promised. `opens_invariants none`/`no_unwind` so it is callable from
// `UserFrame::drop`/`KernelFrame::drop`. The underlying `Inner::free` precondition (`frame.inv()`)
// is discharged in the proving phase from the `FrameAddress` type invariant.
#[verus_spec(result =>
    ensures
        true,
    opens_invariants none
    no_unwind
)]
pub(super) fn free(frame: FrameAddress) -> Result<(), Error> { ... }

///
/// # Description
///
/// Checks whether the frame allocator tracks the frame at the given physical address.
///
/// # Returns
///
/// Returns `true` when the frame allocator tracks the frame at `phys_addr`.
///
// Dependency contract: pure coverage query over the global frame partition. `true` iff the
// allocator tracks the frame (allocated or free), i.e. `phys_view().frames.covers(phys_addr@)`.
// Used by the MMIO boot path to skip frames above RAM that the bitmap does not cover.
#[verus_spec(ret =>
    requires
        phys_addr.inv(),
    ensures
        ret <==> crate::mm::phys::phys_view().frames.covers(phys_addr@),
)]
pub(super) fn is_covered(phys_addr: PageAligned<PhysicalAddress>) -> bool { ... }

/// Reserve a frame so [`alloc`] will skip it.
// Dependency contract: singleton wrapper around `Inner::book`. Reserves a covered, previously
// free frame (refcount becomes 1) so `alloc` never hands it out. The per-frame reservation is
// recorded in the global partition; the booking transition lives in `Inner::book` and is bridged
// to `phys_view().frames` in the proving phase. The boot caller (`book_mmio_regions`) re-derives
// the region-level booking facts via its own lemmas.
#[verus_spec(result =>
    requires
        phys_addr.inv(),
    ensures
        match result {
            Ok(()) => crate::mm::phys::phys_view().frames.reserved(phys_addr@),
            Err(_) => !crate::mm::phys::phys_view().frames.free_frames.contains(phys_addr@),
        },
)]
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> { ... }

/// Book every frame in the given physical memory region.
// Dependency contract: singleton wrapper around `Inner::alloc_range`. On success every frame in
// the region (which must all be free) is reserved with refcount 1. The region-level transition
// lives in `Inner::alloc_range`; the boot caller (`book_physical_memory_regions`) re-derives the
// region-set booking facts via its own lemmas.
#[verus_spec(result =>
    requires
        region.inv(),
    ensures
        match result {
            Ok(()) => crate::mm::phys::phys_view().frames.all_reserved(
                crate::mm::phys::region_frame_addrs(region@.start, region@.size)),
            Err(_) => !crate::mm::phys::phys_view().frames.all_free(
                crate::mm::phys::region_frame_addrs(region@.start, region@.size)),
        },
)]
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error> { ... }

/// Add a new reference to an already-allocated frame (e.g. for copy-on-write sharing).
// Dependency contract: singleton wrapper around `Inner::share`. On success the frame is (still)
// allocated; the per-frame reference-count increment lives in the global partition and is pinned
// to `phys_view().frames` in the proving phase. `external_body` until the free-function layer is
// verified.
#[verus_spec(result =>
    requires
        frame.inv(),
    ensures
        match result {
            Ok(()) => crate::mm::phys::phys_view().frames.allocated_frames.contains(frame@),
            Err(_) => !crate::mm::phys::phys_view().frames.allocated_frames.contains(frame@)
                || crate::mm::phys::phys_view().frames.refcounts[frame@] >= 255,
        },
)]
pub(super) fn share(frame: FrameAddress) -> Result<(), Error> { ... }

/// Returns the current reference count of an already-allocated frame.
// Dependency contract: singleton wrapper around `Inner::refcount`. Reads the current reference
// count of the frame from the global partition (`phys_view().frames`); pure, no mutation.
// `external_body` until the free-function layer is verified.
#[verus_spec(result =>
    requires
        frame.inv(),
    ensures
        match result {
            Ok(count) => {
                &&& crate::mm::phys::phys_view().frames.allocated_frames.contains(frame@)
                &&& count as int == crate::mm::phys::phys_view().frames.refcounts[frame@]
            },
            Err(_) => !crate::mm::phys::phys_view().frames.allocated_frames.contains(frame@),
        },
)]
pub(super) fn refcount(frame: FrameAddress) -> Result<u8, Error> { ... }
