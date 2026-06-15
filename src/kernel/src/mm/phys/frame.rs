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
    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let frame_number: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?}");
                return Err(error);
            },
        };
        // Newly allocated frames have a single owner.
        debug_assert_eq!(self.refcount[frame_number], 0);
        self.refcount[frame_number] = 1;
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                let reason: &str = "frame number is out of bounds";
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Attempt to convert the frame number to a frame address.
        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => Ok(frame_address),
            Err(error) => {
                error!("{error:?}");
                Err(error)
            },
        }
    }

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
    fn alloc_contiguous(&mut self, count: usize) -> Result<FrameAddress, Error> {
        let frame_number: usize = match self.bitmap.alloc_range(count) {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?} (count={count})");
                return Err(error);
            },
        };
        // Newly allocated frames have a single owner.
        for i in frame_number..frame_number + count {
            debug_assert_eq!(self.refcount[i], 0);
            self.refcount[i] = 1;
        }
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                let reason: &str = "frame number is out of bounds";
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => Ok(frame_address),
            Err(error) => {
                error!("{error:?}");
                Err(error)
            },
        }
    }

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
    fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();

        if frame_number >= self.refcount.len() {
            let reason: &str = "frame number out of bounds";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Reject double-frees: the frame must currently have at least one owner.
        if self.refcount[frame_number] == 0 {
            let reason: &str = "frame is already free";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        self.refcount[frame_number] -= 1;

        // Only release the bit in the bitmap when the last owner releases the frame.
        if self.refcount[frame_number] == 0 {
            match self.bitmap.clear(frame_number) {
                Ok(()) => Ok(()),
                Err(error) => {
                    error!("{error:?} (frame={frame:?})");
                    Err(error)
                },
            }
        } else {
            Ok(())
        }
    }

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
    fn share(&mut self, frame: FrameAddress) -> Result<(), Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();

        if frame_number >= self.refcount.len() {
            let reason: &str = "frame number out of bounds";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // The frame must currently have at least one owner. Sharing an unallocated
        // frame is a logic error.
        if self.refcount[frame_number] == 0 {
            let reason: &str = "cannot share an unallocated frame";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        self.refcount[frame_number] = match self.refcount[frame_number].checked_add(1) {
            Some(n) => n,
            None => {
                let reason: &str = "frame reference count overflow";
                error!("{reason} (frame={frame:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        Ok(())
    }

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
    fn refcount(&self, frame: FrameAddress) -> Result<u8, Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();

        if frame_number >= self.refcount.len() {
            let reason: &str = "frame number out of bounds";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        if self.refcount[frame_number] == 0 {
            let reason: &str = "frame is not allocated";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        Ok(self.refcount[frame_number])
    }

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
    fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        let frame_number: usize = phys_addr.into_frame_number().into_raw_value();
        match self.bitmap.set(frame_number) {
            Ok(()) => {
                debug_assert_eq!(self.refcount[frame_number], 0);
                self.refcount[frame_number] = 1;
                Ok(())
            },
            Err(error) => {
                error!("{error:?} (phys_addr={phys_addr:?})");
                Err(error)
            },
        }
    }

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
    fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool {
        let frame_number: usize = phys_addr.into_frame_number().into_raw_value();
        frame_number < self.bitmap.number_of_bits()
    }

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
    ) -> Result<(), Error> {
        let start_frame_number: usize = region.start().into_frame_number().into_raw_value();
        let end_frame_number: usize = start_frame_number + region.size() / mem::FRAME_SIZE - 1;

        // Check that all frames in the range are covered by the bitmap and free,
        // then book them. Uncovered frames indicate a memory layout bug.
        //
        // The coverage check runs unconditionally — including optimized builds —
        // because out-of-bounds indices must be rejected before attempting to set them.
        // This loop runs only at boot when booking memory regions, so the overhead is negligible.
        for index in start_frame_number..=end_frame_number {
            if index >= self.bitmap.number_of_bits() {
                let uncovered_addr: usize = index * mem::FRAME_SIZE;
                let reason: &str = "frame index not covered by the bitmap";
                error!("{} (frame={:#010x}, region={:?})", reason, uncovered_addr, region);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
            match self.bitmap.test(index) {
                Ok(false) => {},
                Ok(true) => {
                    let conflicting_addr: usize = index * mem::FRAME_SIZE;
                    let region_start: usize = region.start().into_raw_value();
                    let region_end: usize = region_start.saturating_add(region.size());
                    let reason: &str = "frame is already allocated";
                    error!(
                        "{} (frame={:#010x}, region_start={:#010x}, region_end={:#010x})",
                        reason, conflicting_addr, region_start, region_end
                    );
                    return Err(Error::new(ErrorCode::OutOfMemory, reason));
                },
                Err(err) => return Err(err),
            }
        }

        // Book all frames in the range.
        for index in start_frame_number..=end_frame_number {
            if let Err(error) = self.bitmap.set(index) {
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
            debug_assert_eq!(self.refcount[index], 0);
            self.refcount[index] = 1;
        }

        Ok(())
    }
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
// Trust boundary (see `verus-ai-logs/tcb-allowed.md`): `instance` reads the
// module-level `static mut INSTANCE` / `INSTANCE_INIT` through
// `unsafe { INSTANCE.assume_init_mut() }`. A Verus `spec fn` cannot read those
// statics, and the verifier does not support `static mut` paths, so the bridge
// between the live singleton and the uninterpreted `phys_view()` is asserted
// here as an external contract: once the allocator is initialized, the returned
// reference is well-formed and its abstract view equals `phys_view().frames`.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_view().initialized,
    ensures
        (*result).inv(),
        (*result)@ == phys_view().frames,
)]
fn instance() -> &'static mut Inner {
    if unlikely(!INSTANCE_INIT.load(ORDER)) {
        panic!("frame allocator used before init()");
    }

    // SAFETY: `INSTANCE_INIT` is `true`, so `INSTANCE` has been fully written by `init()`.
    // The kernel is single-threaded with interrupts disabled, so no concurrent access is possible.
    unsafe { INSTANCE.assume_init_mut() }
}

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
// Skip/exclude from the proof target (see `verus-ai-logs/tcb-allowed.md`): `init`
// materializes the `&'static mut [u8]` refcount table from `static mut
// REFCOUNT_STORAGE` and writes the `static mut INSTANCE`, neither of which the
// verifier supports. Its `#[verus_spec]` contract is honored as a trust boundary.
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
pub(super) unsafe fn init(bitmap: Bitmap) -> Result<(), Error> {
    if unlikely(INSTANCE_INIT.load(ORDER)) {
        return Err(Error::new(ErrorCode::InvalidArgument, "frame allocator already initialized"));
    }

    info!(
        "frame allocator: {} frames, {} MB",
        bitmap.number_of_bits(),
        (bitmap.number_of_bits() * mem::FRAME_SIZE) / constants::MEGABYTE,
    );

    let nframes: usize = bitmap.number_of_bits();
    if nframes > NFRAMES {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "frame bitmap is larger than the configured refcount storage",
        ));
    }

    // SAFETY: single-threaded boot; no other reference to `REFCOUNT_STORAGE` exists.
    let refcount: &'static mut [u8] = unsafe { &mut REFCOUNT_STORAGE[..] };

    // Defensively sync refcounts with any bits already set in the incoming bitmap. The
    // current microvm boot path supplies an empty bitmap and performs all reservations
    // via `book()` / `alloc_range()` after `init()`, so this loop normally does nothing.
    // It is kept as a safety net so a future boot path that hands us a pre-populated
    // bitmap (for example to express firmware-reserved regions) does not silently end
    // up with `bitmap bit = 1, refcount = 0`, which would cause the first `free()` of
    // such a frame to be rejected as a spurious double-free.
    for (i, slot) in refcount.iter_mut().enumerate().take(nframes) {
        if matches!(bitmap.test(i), Ok(true)) {
            *slot = 1;
        }
    }

    // SAFETY: single-threaded boot; no other reference to `INSTANCE` exists.
    unsafe { INSTANCE.write(Inner { bitmap, refcount }) };
    INSTANCE_INIT.store(true, ORDER);
    Ok(())
}

/// Allocate a frame.
#[verus_spec(result =>
    with Tracked(auth): Tracked<&mut PhysAuth>
    requires
        phys_view().initialized,
        phys_view().inv(),
        old(auth)@ == phys_view(),
        old(auth)@.initialized,
        old(auth)@.inv(),
    ensures
        final(auth)@.initialized,
        final(auth)@.inv(),
        // Strong post-state contract carried by the `PhysAuth` token: a successful
        // allocation moves one free frame into `allocated_frames` with refcount 1;
        // an error leaves the allocator unchanged. `old(auth)@` names the pre-state
        // and `final(auth)@` the post-state — the two program points a fixed
        // `phys_view()` constant could not distinguish.
        match result {
            Ok(frame) => {
                &&& frame.inv()
                &&& final(auth)@ == old(auth)@.spec_alloc_one(frame@)
                &&& final(auth)@.frames.allocated_frames.contains(frame@)
                &&& final(auth)@.frames.refcounts[frame@] == 1
            },
            Err(_) => final(auth)@ == old(auth)@,
        },
)]
pub(super) fn alloc() -> Result<FrameAddress, Error> {
    let r = instance();
    let res = r.alloc();
    proof! {
        auth.v.frames = (*r)@;
    }
    res
}

/// # Description
///
/// Allocates `count` physically contiguous frames.
///
/// # Returns
///
/// Returns the base `FrameAddress` of the contiguous range.
///
#[verus_spec(result =>
    with Tracked(auth): Tracked<&mut PhysAuth>
    requires
        phys_view().initialized,
        phys_view().inv(),
        old(auth)@ == phys_view(),
        old(auth)@.initialized,
        old(auth)@.inv(),
        count > 0,
    ensures
        final(auth)@.initialized,
        final(auth)@.inv(),
        // Strong post-state contract carried by the `PhysAuth` token: a successful
        // allocation moves the `count` page-strided frames from the free pool into
        // `allocated_frames`, each with refcount 1; an error leaves the allocator
        // unchanged. Contiguity of the run is the contract of `Inner::alloc_contiguous`
        // (the base address plus page strides); here it is captured as the
        // reserved frame set.
        match result {
            Ok(base) => {
                &&& base.inv()
                &&& ({
                    let frames = Set::new(|addr: int|
                        exists|i: int| 0 <= i < count
                            && addr == #[trigger] (base@ + i * spec_page_size())
                    );
                    &&& final(auth)@ == old(auth)@.spec_alloc_set(frames)
                    &&& frames.subset_of(final(auth)@.frames.allocated_frames)
                })
            },
            Err(_) => final(auth)@ == old(auth)@,
        },
)]
pub(super) fn alloc_contiguous(count: usize) -> Result<FrameAddress, Error> {
    let r = instance();
    let res = r.alloc_contiguous(count);
    proof! {
        auth.v.frames = (*r)@;
    }
    res
}

///
/// # Description
///
/// Returns the number of free frames in the system.
///
/// # Returns
///
/// The number of free frames in the system.
///
#[verus_spec(result =>
    requires
        phys_view().initialized,
        phys_view().inv(),
    ensures
        phys_view().inv(),
        phys_view().initialized,
        // Pure query: returns the size of the free pool. The free set is finite
        // (it is carved from the finite bitmap), which the watermark caller
        // (`check_user_watermark`) relies on to reason with `spec_watermark_ok`.
        phys_view().frames.free_frames.finite(),
        result as int == phys_view().frames.free_frames.len(),
)]
pub(super) fn free_count() -> usize {
    let inner = instance();
    // VERUS DEVIATION (pre-approved: intermediate value for assertions):
    // `number_of_bits() - usage()` is split into named bindings so the proof can
    // recover `num_bits >= 0` from the `usize` result of `number_of_bits()`.
    // `Bitmap::inv()` references the bitmap's private backing slice, so the bound
    // is opaque in this module and cannot be derived inside `lemma_free_count`.
    let nbits: usize = inner.bitmap.number_of_bits();
    let used: usize = inner.bitmap.usage();
    proof! {
        lemma_free_count(inner);
    }
    nbits - used
}

/// Free a frame previously returned by [`alloc`].
//
// Trust boundary (see `verus-ai-logs/tcb-allowed.md`): `free` is reached only from
// `UserFrame::drop` / `KernelFrame::drop`, whose trait-fixed `drop(&mut self)`
// signature is `opens_invariants none` + `no_unwind` and therefore cannot receive
// a `Tracked<&mut PhysAuth>` carrier nor open a global invariant. Without the
// token the body cannot discharge `instance()`'s `phys_view().initialized`
// precondition nor `Inner::free`'s `frame.inv()` precondition, and the contract
// must stay precondition-free so `Drop` is sound. The weak, always-true
// `phys_view().inv()` post-state is honored as an external contract here (the
// logging branch performs no state change). This is the single deliberate,
// caller-justified `Drop`-path exception; the reservation shims (`alloc`,
// `alloc_contiguous`, `book`, `alloc_range`, `share`) carry the strong
// `PhysAuth`-threaded post-state instead.
#[verus_verify(external_body)]
#[verus_spec(result =>
    ensures
        // `free` runs on `Drop` of any `UserFrame`/`KernelFrame`, so it carries no
        // caller precondition. It preserves the subsystem invariant on every path
        // (releasing a reference, and the last reference returns the frame to the
        // free pool). The exact refcount transition is not expressible on this
        // path: `Drop` cannot thread a `PhysAuth` carrier, so there is no
        // `old(auth)@`/`final(auth)@` pair to diff the mutation against.
        phys_view().inv(),
    // `free` is invoked from `Drop` impls (`UserFrame`/`KernelFrame`), which are
    // `no_unwind` and `opens_invariants none`; the shim honours both: it opens no
    // verifier invariants and cannot unwind (errors are returned as values).
    opens_invariants none
    no_unwind
)]
pub(super) fn free(frame: FrameAddress) -> Result<(), Error> {
    instance().free(frame)
}

///
/// # Description
///
/// Checks whether the frame allocator tracks the frame at the given physical address.
///
/// # Returns
///
/// Returns `true` when the frame allocator tracks the frame at `phys_addr`.
///
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
pub(super) fn is_covered(phys_addr: PageAligned<PhysicalAddress>) -> bool {
    instance().is_covered(phys_addr)
}

/// Reserve a frame so [`alloc`] will skip it.
#[verus_spec(result =>
    with Tracked(auth): Tracked<&mut PhysAuth>
    requires
        phys_view().initialized,
        phys_view().inv(),
        old(auth)@ == phys_view(),
        old(auth)@.initialized,
        old(auth)@.inv(),
        phys_addr.inv(),
    ensures
        final(auth)@.initialized,
        final(auth)@.inv(),
        // Strong post-state contract carried by the `PhysAuth` token: a successful
        // booking moves the frame from free to allocated with refcount 1; a
        // failure leaves the allocator unchanged and the frame was not free.
        match result {
            Ok(()) => {
                &&& final(auth)@ == old(auth)@.spec_alloc_one(phys_addr@)
                &&& final(auth)@.frames.allocated_frames.contains(phys_addr@)
                &&& final(auth)@.frames.refcounts[phys_addr@] == 1
            },
            Err(_) => {
                &&& final(auth)@ == old(auth)@
                &&& !old(auth)@.frames.free_frames.contains(phys_addr@)
            },
        },
)]
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
    let r = instance();
    let res = r.book(phys_addr);
    proof! {
        auth.v.frames = (*r)@;
    }
    res
}

/// Book every frame in the given physical memory region.
#[verus_spec(result =>
    with Tracked(auth): Tracked<&mut PhysAuth>
    requires
        phys_view().initialized,
        phys_view().inv(),
        old(auth)@ == phys_view(),
        old(auth)@.initialized,
        old(auth)@.inv(),
        region.inv(),
    ensures
        final(auth)@.initialized,
        final(auth)@.inv(),
        // Strong post-state contract carried by the `PhysAuth` token: on success
        // every frame of the region moves from free to allocated (each refcount 1);
        // on failure the allocator is unchanged and at least one region frame was
        // not free. The region's frame set is `PhysMemView::region_frames`,
        // matching `Inner::alloc_range`.
        match result {
            Ok(()) => {
                &&& final(auth)@ == old(auth)@.spec_alloc_set(
                    PhysMemView::region_frames(region@.start, region@.size))
                &&& PhysMemView::region_frames(region@.start, region@.size)
                    .subset_of(final(auth)@.frames.allocated_frames)
            },
            Err(_) => {
                &&& final(auth)@ == old(auth)@
                &&& !PhysMemView::region_frames(region@.start, region@.size)
                    .subset_of(old(auth)@.frames.free_frames)
            },
        },
)]
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error> {
    let r = instance();
    let res = r.alloc_range(region);
    proof! {
        auth.v.frames = (*r)@;
    }
    res
}

/// Add a new reference to an already-allocated frame (e.g. for copy-on-write sharing).
#[verus_spec(result =>
    with Tracked(auth): Tracked<&mut PhysAuth>
    requires
        phys_view().initialized,
        phys_view().inv(),
        old(auth)@ == phys_view(),
        old(auth)@.initialized,
        old(auth)@.inv(),
        frame.inv(),
    ensures
        final(auth)@.initialized,
        final(auth)@.inv(),
        // Strong post-state contract carried by the `PhysAuth` token: on success
        // the frame gains one reference (the allocated/free partition is unchanged)
        // and remains allocated; on failure the allocator is unchanged and the
        // frame was either not allocated or already at the u8 refcount ceiling.
        match result {
            Ok(()) => {
                &&& final(auth)@ == old(auth)@.spec_share(frame@)
                &&& final(auth)@.frames.allocated_frames.contains(frame@)
                &&& final(auth)@.frames.refcounts.contains_key(frame@)
            },
            Err(_) => {
                &&& final(auth)@ == old(auth)@
                &&& (!old(auth)@.frames.allocated_frames.contains(frame@)
                    || (old(auth)@.frames.refcounts.contains_key(frame@)
                        && old(auth)@.frames.refcounts[frame@] >= 255))
            },
        },
)]
pub(super) fn share(frame: FrameAddress) -> Result<(), Error> {
    let r = instance();
    let res = r.share(frame);
    proof! {
        auth.v.frames = (*r)@;
    }
    res
}

/// Returns the current reference count of an already-allocated frame.
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
pub(super) fn refcount(frame: FrameAddress) -> Result<u8, Error> {
    instance().refcount(frame)
}
