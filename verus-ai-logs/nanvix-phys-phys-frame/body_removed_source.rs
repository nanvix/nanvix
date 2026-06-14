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
    #[verus_verify(external_body)]
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
    #[verus_verify(external_body)]
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
    #[verus_verify(external_body)]
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
    #[verus_verify(external_body)]
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
    #[verus_verify(external_body)]
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
    #[verus_verify(external_body)]
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
    #[verus_verify(external_body)]
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
    #[verus_verify(external_body)]
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
#[verus_verify(external_body)]
#[verus_spec(result =>
    ensures
        // `init` establishes the subsystem invariant. On success the allocator is
        // marked initialized; on either outcome the abstract view is well-formed
        // (vacuously so if it never became initialized).
        phys_view().inv(),
        match result {
            Ok(()) => phys_view().initialized,
            Err(_) => true,
        },
)]
pub(super) unsafe fn init(bitmap: Bitmap) -> Result<(), Error> { ... }

/// Allocate a frame.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
        phys_view().inv(),
    ensures
        phys_view().inv(),
        phys_view().initialized,
        // On success the returned frame is freshly reserved: page-aligned, now in
        // `allocated_frames`, and carrying a single reference. On failure nothing
        // is reported about the returned (absent) frame.
        match result {
            Ok(frame) => {
                &&& frame.inv()
                &&& phys_view().frames.allocated_frames.contains(frame@)
                &&& phys_view().frames.refcounts.contains_key(frame@)
                &&& phys_view().frames.refcounts[frame@] == 1
            },
            Err(_) => true,
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
pub(super) fn free_count() -> usize { ... }

/// Free a frame previously returned by [`alloc`].
#[verus_verify(external_body)]
#[verus_spec(result =>
    ensures
        // `free` runs on `Drop` of any `UserFrame`/`KernelFrame`, so it carries no
        // caller precondition. It preserves the subsystem invariant on every path
        // (releasing a reference, and the last reference returns the frame to the
        // free pool). The exact refcount transition is not expressible here:
        // `phys_view()` is a single fixed value, with no `old(phys_view())` to
        // compare against.
        phys_view().inv(),
    // `free` is invoked from `Drop` impls (`UserFrame`/`KernelFrame`), which are
    // `no_unwind` and `opens_invariants none`; the shim honours both: it opens no
    // verifier invariants and cannot unwind (errors are returned as values).
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
#[verus_verify(external_body)]
#[verus_spec(ret =>
    requires
        phys_view().initialized,
        phys_addr.inv(),
    ensures
        phys_view().inv(),
        // `is_covered` reports exactly the frames the allocator tracks: the union
        // of reserved and free frames (`PhysMemView::covered`).
        ret <==> phys_view().covered().contains(phys_addr@),
)]
pub(super) fn is_covered(phys_addr: PageAligned<PhysicalAddress>) -> bool { ... }

/// Reserve a frame so [`alloc`] will skip it.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
        phys_addr.inv(),
    ensures
        phys_view().inv(),
        // On success the frame is reserved (now in `allocated_frames`); the
        // subsystem stays initialized and well-formed regardless of outcome.
        phys_view().initialized,
        match result {
            Ok(()) => phys_view().frames.allocated_frames.contains(phys_addr@),
            Err(_) => true,
        },
)]
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> { ... }

/// Book every frame in the given physical memory region.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
        region.inv(),
    ensures
        phys_view().inv(),
        phys_view().initialized,
        // On success every frame of the region is reserved. The region's frame
        // set is `PhysMemView::region_frames` and matches `Inner::alloc_range`.
        match result {
            Ok(()) => forall|a: int|
                #[trigger] PhysMemView::region_frames(region@.start, region@.size).contains(a)
                    ==> phys_view().frames.allocated_frames.contains(a),
            Err(_) => true,
        },
)]
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error> { ... }

/// Add a new reference to an already-allocated frame (e.g. for copy-on-write sharing).
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
        phys_view().inv(),
        frame.inv(),
    ensures
        phys_view().inv(),
        phys_view().initialized,
        // On success the frame remains allocated (it has gained a reference). The
        // increment itself is not stated: `phys_view()` is a single fixed value
        // with no `old(phys_view())` to compare against.
        match result {
            Ok(()) => {
                &&& phys_view().frames.allocated_frames.contains(frame@)
                &&& phys_view().frames.refcounts.contains_key(frame@)
            },
            Err(_) => true,
        },
)]
pub(super) fn share(frame: FrameAddress) -> Result<(), Error> { ... }

/// Returns the current reference count of an already-allocated frame.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
        phys_view().inv(),
        frame.inv(),
    ensures
        phys_view().inv(),
        phys_view().initialized,
        // A pure query: on success it returns the frame's current refcount (and
        // the frame is allocated); on failure the frame is not allocated.
        match result {
            Ok(count) => {
                &&& phys_view().frames.allocated_frames.contains(frame@)
                &&& phys_view().frames.refcounts.contains_key(frame@)
                &&& count as int == phys_view().frames.refcounts[frame@]
            },
            Err(_) => !phys_view().frames.allocated_frames.contains(frame@),
        },
)]
pub(super) fn refcount(frame: FrameAddress) -> Result<u8, Error> { ... }
