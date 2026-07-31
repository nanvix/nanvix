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
                    &&& old(self)@.is_free(frame@)
                    &&& final(self)@ == FrameAllocView {
                        refcounts: old(self)@.refcounts.insert(frame@, 1int),
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& old(self)@.no_free_frames()
                }
            },
    )]
    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        proof_decl! {
            let ghost g_old = self@;
            let ghost pre_sb = self.bitmap@.set_bits;
            let ghost pre_nb = self.bitmap@.num_bits;
            let ghost pre_rc = self.refcount@;
        }
        proof! {
            // Snapshot the pre-state view and expose the bitmap/refcount invariant link so later
            // proof obligations can read it off the captured component snapshots.
            lemma_capture_inv_facts(self, g_old, pre_sb, pre_nb, pre_rc);
        }
        let frame_number: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                proof! {
                    // The bitmap is full (every bit set), so every covered frame has refcount > 0
                    // and no covered frame is free.
                    lemma_alloc_full_no_free(self, g_old, pre_sb, pre_nb, pre_rc);
                }
                error!("{error:?}");
                return Err(error);
            },
        };
        proof_decl! { let ghost idx = frame_number as int; }
        proof! {
            // Representability of the new in-range index (from the captured invariant).
            lemma_alloc_index_repr(idx, pre_nb);
        }
        // Newly allocated frames have a single owner.
        #[cfg(not(verus_keep_ghost))]
        debug_assert_eq!(self.refcount[frame_number], 0);
        self.refcount[frame_number] = 1;
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                proof! {
                    // `None` requires `idx > spec_max`, contradicting representability. Unreachable.
                    assert(false);
                }
                let reason: &str = "frame number is out of bounds";
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Attempt to convert the frame number to a frame address.
        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => {
                proof! {
                    lemma_post_reserve_one_by_index(self, idx, g_old, pre_sb, pre_nb, pre_rc);
                }
                Ok(frame_address)
            },
            Err(error) => {
                proof! {
                    // `from_frame_number` is total (always `Ok`); this arm is unreachable.
                    assert(false);
                }
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
                        let frame_indices = Set::range(0, count as int);
                        let frames = frame_indices.map_by(
                            |i: int| base@ + i * spec_page_size(),
                            |addr: int| (addr - base@) / spec_page_size(),
                        );
                        &&& old(self)@.all_free(frames)
                        &&& final(self)@ == FrameAllocView {
                            refcounts: old(self)@.refcounts.union_prefer_right(
                                Map::new(frames, |addr: int| 1int)
                            ),
                        }
                    })
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& !old(self)@.exists_contiguous_free_run(count as int)
                }
            },
    )]
    fn alloc_contiguous(&mut self, count: usize) -> Result<FrameAddress, Error> {
        proof_decl! {
            let ghost g_old = self@;
            let ghost pre_sb = self.bitmap@.set_bits;
            let ghost pre_nb = self.bitmap@.num_bits;
            let ghost pre_rc = self.refcount@;
        }
        proof! {
            // Snapshot the pre-state view and expose the bitmap/refcount invariant link so later
            // proof obligations can read it off the captured component snapshots.
            lemma_capture_inv_facts(self, g_old, pre_sb, pre_nb, pre_rc);
        }
        // Reject impossible requests before delegating to the bitmap range allocator.
        if count > self.bitmap.number_of_bits() {
            proof! {
                // With fewer than `count` bits, no run of `count` clear bits can exist.
                assert forall|start: int|
                    #![trigger self.bitmap@.has_free_range_at(start, count as int)]
                    !self.bitmap@.has_free_range_at(start, count as int) by {}
                lemma_no_bitmap_range_implies_no_free_run(self, count as int);
            }
            let reason: &str = "requested range exceeds bitmap size";
            error!("{reason} (count={count}, bitmap_bits={})", self.bitmap.number_of_bits());
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        let frame_number: usize = match self.bitmap.alloc_range(count) {
            Ok(index) => index,
            Err(error) => {
                proof! {
                    // `alloc_range` failed: no run of `count` clear bits exists in the bitmap.
                    lemma_no_bitmap_range_implies_no_free_run(self, count as int);
                }
                error!("{error:?} (count={count})");
                return Err(error);
            },
        };
        proof_decl! { let ghost start = frame_number as int; }
        // `alloc_range` Ok guarantees the booked range was entirely clear beforehand.
        proof! {
            assert forall|j: int| start <= j < start + count as int implies !pre_sb.contains(j) by {
                assert(!old(self).bitmap@.is_bit_set(j));
            }
        }
        // Newly allocated frames have a single owner.
        #[verus_spec(
            invariant
                start == frame_number as int,
                0 <= start,
                start + count as int <= pre_nb,
                pre_nb <= pre_rc.len(),
                self.bitmap@.num_bits == pre_nb,
                self.bitmap.inv(),
                self.bitmap@.set_bits
                    == pre_sb.union(BitmapView::range_set(start, start + count as int)),
                self.refcount@.len() == pre_rc.len(),
                forall|j: int| start <= j < start + count as int ==> !pre_sb.contains(j),
                forall|k: int| 0 <= k < pre_rc.len() ==>
                    #[trigger] self.refcount@[k] == (if start <= k < i as int {
                        1u8
                    } else {
                        pre_rc[k]
                    }),
            decreases frame_number + count - i,
        )]
        for i in frame_number..frame_number + count {
            #[cfg(not(verus_keep_ghost))]
            debug_assert_eq!(self.refcount[i], 0);
            self.refcount[i] = 1;
        }
        proof! {
            // Re-establish `internal_inv` after range booking: the bitmap has the whole range set,
            // and the refcount loop set exactly the range's slots to 1.
            lemma_reestablish_inv_range(self, pre_sb, pre_nb, pre_rc, start, start + count as int);
            // Representability: `start < pre_nb`, so `old(self)`'s internal_inv bounds `start`.
            lemma_alloc_index_repr(start, pre_nb);
        }
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                proof! {
                    // `None` requires `start > spec_max`, contradicting representability. Unreachable.
                    assert(false);
                }
                let reason: &str = "frame number is out of bounds";
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => {
                proof! {
                    let base = frame_address@;
                    lemma_alloc_contiguous_post(self, base, start, count as int, g_old, pre_sb, pre_nb, pre_rc);
                }
                Ok(frame_address)
            },
            Err(error) => {
                proof! {
                    // `from_frame_number` is total (always `Ok`); this arm is unreachable.
                    assert(false);
                }
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
    #[verus_spec(result =>
        requires
            old(self).inv(),
            frame.inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.is_allocated(frame@)
                    // Releasing one reference simply decrements the count. When it
                    // reaches zero the frame stays covered but becomes free, so the
                    // map domain is unchanged in both the shared and last-owner cases.
                    &&& final(self)@ == FrameAllocView {
                        refcounts: old(self)@.refcounts.insert(
                            frame@, old(self)@.refcounts[frame@] - 1
                        ),
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& !old(self)@.is_allocated(frame@)
                }
            },
    )]
    fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();
        proof_decl! {
            let ghost g_old = self@;
            let ghost pre_sb = self.bitmap@.set_bits;
            let ghost pre_nb = self.bitmap@.num_bits;
            let ghost pre_rc = self.refcount@;
        }
        proof! {
            lemma_free_pre(self, frame@);
        }

        if frame_number >= self.refcount.len() {
            proof! {
                // frame_number >= refcount.len() >= num_bits, so the bit is clear: not allocated.
                lemma_refcount_err_bit_clear(self, frame_number as int);
            }
            let reason: &str = "frame number out of bounds";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Reject double-frees: the frame must currently have at least one owner.
        if self.refcount[frame_number] == 0 {
            proof! {
                // refcount[fnx] == 0, so the bit is clear (internal_inv), hence not allocated.
                lemma_refcount_err_bit_clear(self, frame_number as int);
            }
            let reason: &str = "frame is already free";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // The frame is currently allocated: refcount[fnx] > 0 and fnx < num_bits, so the bit is set.
        proof! {
            lemma_frame_allocated(self, frame@, frame_number as int);
        }

        self.refcount[frame_number] -= 1;

        // Only release the bit in the bitmap when the last owner releases the frame.
        if self.refcount[frame_number] == 0 {
            match self.bitmap.clear(frame_number) {
                Ok(()) => {
                    proof! {
                        lemma_post_release_one(self, frame@, frame_number as int, g_old, pre_sb, pre_nb, pre_rc);
                    }
                    Ok(())
                },
                Err(error) => {
                    proof! {
                        // Unreachable: the bit is set and in range, so `clear` returns `Ok`.
                        assert(false);
                    }
                    error!("{error:?} (frame={frame:?})");
                    Err(error)
                },
            }
        } else {
            // Still shared: only the refcount slot changed (decremented by one); the
            // allocated/free partition is unchanged.
            proof! {
                let nv = self.refcount@[frame_number as int];
                lemma_post_update_slot(self, frame@, frame_number as int, nv, g_old, pre_sb, pre_nb, pre_rc);
            }
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
    #[verus_spec(result =>
        requires
            old(self).inv(),
            frame.inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.is_allocated(frame@)
                    // The concrete `checked_add` rejects sharing at the u8 ceiling, so
                    // success implies headroom. Stating it explicitly keeps `Ok`/`Err`
                    // complementary and lets `final(self)@.wf()` follow directly (the new
                    // count is `<= 255`) instead of via a contradiction with `inv()`.
                    &&& old(self)@.refcounts[frame@] < 255
                    &&& final(self)@ == FrameAllocView {
                        refcounts: old(self)@.refcounts.insert(
                            frame@, old(self)@.refcounts[frame@] + 1
                        ),
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& old(self)@.is_allocated(frame@) ==> old(self)@.refcounts[frame@] >= 255
                }
            },
    )]
    fn share(&mut self, frame: FrameAddress) -> Result<(), Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();
        proof_decl! {
            let ghost g_old = self@;
            let ghost pre_sb = self.bitmap@.set_bits;
            let ghost pre_nb = self.bitmap@.num_bits;
            let ghost pre_rc = self.refcount@;
        }
        proof! {
            lemma_free_pre(self, frame@);
        }

        if frame_number >= self.refcount.len() {
            proof! {
                // frame_number >= refcount.len() >= num_bits, so the bit is clear: not allocated.
                lemma_refcount_err_bit_clear(self, frame_number as int);
            }
            let reason: &str = "frame number out of bounds";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // The frame must currently have at least one owner. Sharing an unallocated
        // frame is a logic error.
        if self.refcount[frame_number] == 0 {
            proof! {
                // refcount[fnx] == 0, so the bit is clear (internal_inv), hence not allocated.
                lemma_refcount_err_bit_clear(self, frame_number as int);
            }
            let reason: &str = "cannot share an unallocated frame";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        proof! {
            // The frame is allocated: refcount[fnx] > 0 and fnx < num_bits, so the bit is set.
            lemma_frame_allocated(self, frame@, frame_number as int);
        }

        self.refcount[frame_number] = match self.refcount[frame_number].checked_add(1) {
            Some(n) => n,
            None => {
                // Overflow: the old refcount was at its u8 maximum (255). The state is
                // unchanged, satisfying the Err arm's refcount-saturated disjunct.
                proof! {
                    lemma_share_overflow(self, frame@, frame_number as int, g_old);
                }
                let reason: &str = "frame reference count overflow";
                error!("{reason} (frame={frame:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Only the refcount slot changed (incremented by one); the allocated/free partition
        // is unchanged.
        proof! {
            let nv = self.refcount@[frame_number as int];
            lemma_post_update_slot(self, frame@, frame_number as int, nv, g_old, pre_sb, pre_nb, pre_rc);
        }

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
    #[verus_spec(result =>
        requires
            self.inv(),
            frame.inv(),
        ensures
            self.inv(),
            match result {
                Ok(count) => {
                    &&& self@.is_allocated(frame@)
                    &&& count as int == self@.refcounts[frame@]
                },
                Err(_) => {
                    !self@.is_allocated(frame@)
                }
            },
    )]
    fn refcount(&self, frame: FrameAddress) -> Result<u8, Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();
        proof! {
            vstd::arithmetic::div_mod::lemma_div_pos_is_pos(frame@, spec_page_size());
            lemma_alloc_contains(self, frame@);
        }

        if frame_number >= self.refcount.len() {
            proof! {
                // frame_number >= refcount.len() >= num_bits, so the bit is clear: not allocated.
                lemma_refcount_err_bit_clear(self, frame_number as int);
            }
            let reason: &str = "frame number out of bounds";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        if self.refcount[frame_number] == 0 {
            proof! {
                // refcount[i] == 0, so the bit is clear (internal_inv), hence not allocated.
                lemma_refcount_err_bit_clear(self, frame_number as int);
            }
            let reason: &str = "frame is not allocated";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        proof! {
            // refcount[i] != 0 and i < refcount.len(); tail-zero forces i < num_bits, so the
            // bit is set, the frame is allocated, and its refcount-map value is the slot value.
            lemma_frame_allocated(self, frame@, frame_number as int);
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
    #[verus_spec(result =>
        requires
            old(self).inv(),
            phys_addr.inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.is_free(phys_addr@)
                    &&& final(self)@ == FrameAllocView {
                        refcounts: old(self)@.refcounts.insert(phys_addr@, 1int),
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& !old(self)@.is_free(phys_addr@)
                }
            },
    )]
    fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        let frame_number: usize = phys_addr.into_frame_number().into_raw_value();
        proof_decl! {
            let ghost g_old = self@;
            let ghost pre_sb = self.bitmap@.set_bits;
            let ghost pre_nb = self.bitmap@.num_bits;
            let ghost pre_rc = self.refcount@;
        }
        proof! {
            lemma_book_pre(self, phys_addr@);
        }
        match self.bitmap.set(frame_number) {
            Ok(()) => {
                #[cfg(not(verus_keep_ghost))]
                debug_assert_eq!(self.refcount[frame_number], 0);
                self.refcount[frame_number] = 1;
                proof! {
                    lemma_post_reserve_one(self, phys_addr@, frame_number as int, g_old, pre_sb, pre_nb, pre_rc);
                }
                Ok(())
            },
            Err(error) => {
                // `set()` failed: the bit was already set or out of range, so the frame is not
                // free in `old(self)`.
                proof! {
                    lemma_book_set_failed(self, phys_addr@, g_old);
                }
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
    #[verus_spec(ret =>
        requires
            self.inv(),
            phys_addr.inv(),
        ensures
            self.inv(),
            ret <==> self@.is_covered(phys_addr@),
    )]
    fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool {
        let frame_number: usize = phys_addr.into_frame_number().into_raw_value();
        proof! {
            vstd::arithmetic::div_mod::lemma_div_pos_is_pos(phys_addr@, spec_page_size());
            lemma_covered_iff(self, phys_addr@);
        }
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
    #[verus_spec(result =>
        requires
            old(self).inv(),
            region.inv(),
        ensures
            final(self).inv(),
            ({
                let frames = region_frame_addrs(region@.start, region@.size);
                match result {
                    Ok(()) => {
                        &&& old(self)@.all_free(frames)
                        &&& final(self)@ == FrameAllocView {
                            refcounts: old(self)@.refcounts.union_prefer_right(
                                Map::new(frames, |addr: int| 1int)
                            ),
                        }
                    },
                    Err(_) => {
                        &&& final(self)@ == old(self)@
                        &&& !old(self)@.all_free(frames)
                    },
                }
            }),
    )]
    fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error> {
        let start_frame_number: usize = region.start().into_frame_number().into_raw_value();
        let nframes: usize = region.size() / mem::FRAME_SIZE;
        proof_decl! {
            let ghost g_old = self@;
            let ghost pre_sb = self.bitmap@.set_bits;
            let ghost pre_nb = self.bitmap@.num_bits;
            let ghost pre_rc = self.refcount@;
            let ghost ps = spec_page_size();
            let ghost rstart = region@.start;
            let ghost rsize = region@.size;
            let ghost start_fn = start_frame_number as int;
            let ghost nfr = nframes as int;
        }
        proof! {
            lemma_capture_inv_facts(self, g_old, pre_sb, pre_nb, pre_rc);
            assert(ps == 4096);
            lemma_alloc_range_geometry(rstart, rsize, ps, start_fn, nfr);
        }
        let end_exclusive: usize = start_frame_number + nframes;
        proof! {
            assert(end_exclusive as int == start_fn + nfr);
        }

        // Check that all frames in the range are covered by the bitmap and free,
        // then book them. Uncovered frames indicate a memory layout bug.
        //
        // The coverage check runs unconditionally — including optimized builds —
        // because out-of-bounds indices must be rejected before attempting to set them.
        // This loop runs only at boot when booking memory regions, so the overhead is negligible.
        let mut index: usize = start_frame_number;
        #[verus_spec(
            invariant
                start_frame_number <= index,
                index <= end_exclusive,
                start_fn == start_frame_number as int,
                end_exclusive as int == start_fn + nfr,
                nfr >= 1,
                ps == spec_page_size(),
                rstart == region@.start,
                rsize == region@.size,
                rstart / ps == start_fn,
                (rstart + rsize) / ps == start_fn + nfr,
                self@ == g_old,
                self@ == old(self)@,
                self.bitmap@.set_bits == pre_sb,
                self.bitmap@.num_bits == pre_nb,
                self.refcount@ == pre_rc,
                self.bitmap.inv(),
                self.internal_inv(),
                forall|k: int| start_fn <= k < index as int ==>
                    #[trigger] pre_sb.contains(k) == false && k < pre_nb,
            decreases end_exclusive - index,
        )]
        while index < end_exclusive {
            if index >= self.bitmap.number_of_bits() {
                // `index` is out of range here, so `index * FRAME_SIZE` can overflow `usize`
                // (e.g. on 32-bit targets), panicking in debug builds on the very error path
                // meant to report the problem. Saturate: the value only feeds a diagnostic.
                proof! {
                    // This frame is in the requested range but not covered, so it cannot be free.
                    lemma_range_uncovered_not_all_free(self, index as int, rstart, rsize, ps, start_fn, nfr, g_old);
                }
                let reason: &str = "frame index not covered by the bitmap";
                error!(
                    "{} (frame={:#010x}, region={:?})",
                    reason,
                    index.saturating_mul(mem::FRAME_SIZE),
                    region
                );
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
            match self.bitmap.test(index) {
                Ok(false) => {
                    proof! {
                        // Record coverage of `index` so the loop invariant extends to `index + 1`.
                        assert(!pre_sb.contains(index as int));
                        assert((index as int) < pre_nb);
                    }
                },
                Ok(true) => {
                    proof! {
                        // This frame is in the requested range but already allocated, not free.
                        lemma_range_allocated_not_all_free(self, index as int, rstart, rsize, ps, start_fn, nfr, g_old);
                    }
                    let region_start: usize = region.start().into_raw_value();
                    let region_end: usize = region_start.saturating_add(region.size());
                    let reason: &str = "frame is already allocated";
                    error!(
                        "{} (frame={:#010x}, region_start={:#010x}, region_end={:#010x})",
                        reason,
                        index.saturating_mul(mem::FRAME_SIZE),
                        region_start,
                        region_end
                    );
                    return Err(Error::new(ErrorCode::OutOfMemory, reason));
                },
                Err(err) => {
                    proof! {
                        // `index < num_bits` was checked above, so `test` cannot fail. Unreachable.
                        assert(index < self.bitmap@.num_bits);
                        assert(false);
                    }
                    return Err(err);
                },
            }
            index += 1;
        }
        // The loop's exit invariant covers every index in `[start_fn, start_fn + nfr)`:
        // each frame is free in the pre-state bitmap and lies within bounds.
        proof! {
            assert(pre_sb.contains(start_fn + nfr - 1) == false);
            assert(start_fn + nfr <= pre_nb);
            assert forall|j: int| start_fn <= j < start_fn + nfr implies !pre_sb.contains(j) by {}
        }

        // Book all frames in the range.
        #[verus_spec(
            invariant
                start_fn == start_frame_number as int,
                end_exclusive as int == start_fn + nfr,
                start_fn + nfr <= pre_nb,
                pre_nb <= pre_rc.len(),
                self.bitmap.inv(),
                self.bitmap@.num_bits == pre_nb,
                self.bitmap@.set_bits == pre_sb.union(BitmapView::range_set(start_fn, index as int)),
                self.refcount@.len() == pre_rc.len(),
                forall|j: int| start_fn <= j < start_fn + nfr ==> !pre_sb.contains(j),
                forall|k: int| 0 <= k < pre_rc.len() ==>
                    #[trigger] self.refcount@[k] == (if start_fn <= k < index as int {
                        1u8
                    } else {
                        pre_rc[k]
                    }),
            decreases end_exclusive - index,
        )]
        for index in start_frame_number..end_exclusive {
            if let Err(error) = self.bitmap.set(index) {
                // The bit at `index` is still clear and in range, so `set` cannot fail.
                proof! {
                    assert(!BitmapView::range_set(start_fn, index as int).contains(index as int));
                    assert(!pre_sb.contains(index as int));
                    assert(false);
                }
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
            #[cfg(not(verus_keep_ghost))]
            debug_assert_eq!(self.refcount[index], 0);
            self.refcount[index] = 1;
        }
        proof! {
            // Re-establish `internal_inv` after booking, prove the requested range was free, and
            // reconstruct the post-state view as the pre-state map merged with the booked run.
            lemma_alloc_range_post(self, rstart, rsize, ps, start_fn, nfr, g_old, pre_sb, pre_nb, pre_rc);
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
pub(super) fn alloc() -> Result<FrameAddress, Error> {
    instance().alloc()
}

/// # Description
///
/// Allocates `count` physically contiguous frames.
///
/// # Returns
///
/// Returns the base `FrameAddress` of the contiguous range.
///
pub(super) fn alloc_contiguous(count: usize) -> Result<FrameAddress, Error> {
    instance().alloc_contiguous(count)
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
pub(super) fn free_count() -> usize {
    let inner = instance();
    inner.bitmap.number_of_bits() - inner.bitmap.usage()
}

/// Free a frame previously returned by [`alloc`].
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
pub(super) fn is_covered(phys_addr: PageAligned<PhysicalAddress>) -> bool {
    instance().is_covered(phys_addr)
}

/// Reserve a frame so [`alloc`] will skip it.
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
    instance().book(phys_addr)
}

/// Book every frame in the given physical memory region.
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error> {
    instance().alloc_range(region)
}

/// Add a new reference to an already-allocated frame (e.g. for copy-on-write sharing).
pub(super) fn share(frame: FrameAddress) -> Result<(), Error> {
    instance().share(frame)
}

/// Returns the current reference count of an already-allocated frame.
pub(super) fn refcount(frame: FrameAddress) -> Result<u8, Error> {
    instance().refcount(frame)
}
