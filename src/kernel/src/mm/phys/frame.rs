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
    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        proof_decl! { let ghost old_self = *self; }
        let frame_number: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                proof! {
                    // `alloc` failed: the bitmap is full and its view is unchanged, so the whole
                    // allocator view is unchanged and there are no free frames.
                    lemma_view_determined(self, &old_self);
                    lemma_full_no_free(self);
                }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?}");
                return Err(error);
            },
        };
        // Newly allocated frames have a single owner.
        #[cfg(not(verus_keep_ghost))]
        debug_assert_eq!(self.refcount[frame_number], 0);
        // The allocated index is in range and representable as a frame number; the proof below
        // discharges the conversions that follow.
        proof_decl! { let ghost pa: int = frame_number as int * spec_page_size(); }
        proof! {
            let idx = frame_number as int;
            assert(0 <= idx < old_self.bitmap@.num_bits);
            assert(!old_self.bitmap@.set_bits.contains(idx));
            assert(self.bitmap@.set_bits == old_self.bitmap@.set_bits.insert(idx));
            assert(self.bitmap@.num_bits == old_self.bitmap@.num_bits);
            // The freshly allocated slot was zero before this call.
            assert(old_self.refcount@[idx] == 0);
            // `pa` is the frame's base address.
            assert(pa == frame_addr_of(idx));
            vstd::arithmetic::div_mod::lemma_mod_multiples_basic(idx, spec_page_size());
            assert(pa % spec_page_size() == 0);
            assert(pa >= 0) by (nonlinear_arith) requires idx >= 0, spec_page_size() > 0, pa == idx * spec_page_size();
            lemma_frame_facts(&old_self, pa, idx);
            assert(old_self@.free_frames.contains(pa));
            // The index is a representable frame number: `internal_inv` carries
            // `num_bits <= spec_max() + 1`, and `idx < num_bits`, hence `idx <= spec_max()`.
            assert(idx < self.bitmap@.num_bits);
            assert(self.bitmap@.num_bits <= FrameNumber::spec_max() + 1);
            assert(idx <= FrameNumber::spec_max());
        }
        self.refcount[frame_number] = 1;
        proof! {
            let idx = frame_number as int;
            assert(self.refcount@ == old_self.refcount@.update(idx, 1));
            lemma_refcount_book(&old_self, self, idx, pa);
            lemma_internal_inv_implies_wf(self);
        }
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                proof! { assert(false); }
                let reason: &str = "frame number is out of bounds";
                #[cfg(not(verus_keep_ghost))]
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Attempt to convert the frame number to a frame address.
        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => {
                proof! {
                    assert(frame_address@ == pa);
                }
                Ok(frame_address)
            },
            Err(error) => {
                #[cfg(not(verus_keep_ghost))]
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
        proof_decl! { let ghost old_self = *self; }
        // `Bitmap::alloc_range` requires `count <= num_bits`. Reject oversized requests up front;
        // this leaves the allocator state untouched, satisfying the `Err` contract.
        let nbits: usize = self.bitmap.number_of_bits();
        if count > nbits {
            proof! { lemma_view_determined(self, &old_self); }
            let reason: &str = "contiguous request exceeds bitmap capacity";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason:?} (count={count})");
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }
        let frame_number: usize = match self.bitmap.alloc_range(count) {
            Ok(index) => index,
            Err(error) => {
                proof! { lemma_view_determined(self, &old_self); }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (count={count})");
                return Err(error);
            },
        };
        proof! {
            // Bits `[frame_number, frame_number + count)` were free and are now set; the refcount
            // slice still matches `old_self` (only the bitmap changed).
            assert(self.refcount@ == old_self.refcount@);
            assert(self.bitmap@.num_bits == old_self.bitmap@.num_bits);
            assert(self.bitmap@.set_bits
                == old_self.bitmap@.set_bits.union(
                    ::bitmap::BitmapView::range_set(
                        frame_number as int, frame_number as int + count as int)));
            assert(0 <= frame_number);
            assert(frame_number as int + count as int <= self.bitmap@.num_bits);
            assert(old_self.refcount@.len() >= old_self.bitmap@.num_bits);
            // The range was unset in `old_self`, so booking it does not clobber an owner.
            assert forall|j: int|
                frame_number as int <= j < frame_number as int + count as int implies
                !old_self.bitmap@.set_bits.contains(j) by {
                assert(old_self.bitmap@.all_bits_unset_in_range(
                    frame_number as int, frame_number as int + count as int));
                assert(!old_self.bitmap@.is_bit_set(j));
            }
        }
        proof_decl! { let ghost lo: int = frame_number as int; }
        proof_decl! { let ghost hi: int = frame_number as int + count as int; }
        // Newly allocated frames have a single owner.
        #[cfg_attr(verus_keep_ghost, verus_spec(
            invariant
                lo == frame_number as int,
                hi == frame_number as int + count as int,
                lo <= i <= hi,
                self.bitmap.inv(),
                self.bitmap@.num_bits == old_self.bitmap@.num_bits,
                self.bitmap@.set_bits == old_self.bitmap@.set_bits.union(
                    ::bitmap::BitmapView::range_set(lo, hi)),
                self.refcount@.len() == old_self.refcount@.len(),
                self.refcount@.len() >= self.bitmap@.num_bits,
                hi <= self.bitmap@.num_bits,
                forall|j: int| lo <= j < i ==> self.refcount@[j] == 1,
                forall|k: int| 0 <= k < self.refcount@.len() && !(lo <= k < i)
                    ==> self.refcount@[k] == old_self.refcount@[k],
        ))]
        for i in frame_number..frame_number + count {
            #[cfg(not(verus_keep_ghost))]
            debug_assert_eq!(self.refcount[i], 0);
            proof_decl! { let ghost loop_old = *self; }
            self.refcount[i] = 1;
            proof! {
                assert(self.refcount@ == loop_old.refcount@.update(i as int, 1));
                assert(self.bitmap == loop_old.bitmap);
            }
        }
        proof! {
            assert(self.refcount@.len() == old_self.refcount@.len());
            assert forall|j: int| lo <= j < hi implies self.refcount@[j] == 1 by {}
            assert forall|k: int| 0 <= k < self.refcount@.len() && !(lo <= k < hi) implies
                self.refcount@[k] == old_self.refcount@[k] by {}
            lemma_book_range(&old_self, self, lo, hi);
            lemma_internal_inv_implies_wf(self);
        }
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                proof! {
                    // `frame_number < num_bits <= spec_max() + 1`, so `frame_number <= spec_max()`
                    // and the conversion is total. The bound comes from `internal_inv`, not the
                    // word size, so it holds on every target.
                    assert(frame_number < nbits);
                    assert(nbits as int == self.bitmap@.num_bits);
                    assert(self.bitmap@.num_bits <= FrameNumber::spec_max() + 1);
                    assert(frame_number as int <= FrameNumber::spec_max());
                    assert(false);
                }
                let reason: &str = "frame number is out of bounds";
                #[cfg(not(verus_keep_ghost))]
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => {
                proof! {
                    let base = frame_address@;
                    assert(base == lo * spec_page_size());
                    // The contract's `frames` set coincides with `frame_set(lo, hi)`.
                    let frames = Set::new(|addr: int|
                        exists|i: int| 0 <= i < count as int
                            && addr == #[trigger] (base + i * spec_page_size()));
                    assert(frames =~= frame_set(lo, hi)) by {
                        assert forall|addr: int| frames.contains(addr) implies
                            #[trigger] frame_set(lo, hi).contains(addr) by {
                            let i = choose|i: int| 0 <= i < count as int
                                && addr == #[trigger] (base + i * spec_page_size());
                            assert(addr == (lo + i) * spec_page_size()) by (nonlinear_arith)
                                requires base == lo * spec_page_size(),
                                    addr == base + i * spec_page_size();
                            assert(lo <= lo + i < hi);
                            assert(addr == frame_addr_of(lo + i));
                        }
                        assert forall|addr: int| frame_set(lo, hi).contains(addr) implies
                            #[trigger] frames.contains(addr) by {
                            let j = choose|j: int| lo <= j < hi
                                && addr == #[trigger] frame_addr_of(j);
                            assert(addr == (lo + (j - lo)) * spec_page_size());
                            assert(addr == base + (j - lo) * spec_page_size()) by (nonlinear_arith)
                                requires base == lo * spec_page_size(),
                                    addr == j * spec_page_size();
                            assert(0 <= j - lo < count as int);
                        }
                    }
                    // The booked range was free in the pre-state, so `frames` is a subset of
                    // the old free-frame set.
                    assert(old_self.bitmap@.all_bits_unset_in_range(lo, hi));
                    assert forall|addr: int| frame_set(lo, hi).contains(addr) implies
                        #[trigger] old_self@.free_frames.contains(addr) by {
                        let j = choose|j: int| lo <= j < hi
                            && addr == #[trigger] frame_addr_of(j);
                        assert(!old_self.bitmap@.is_bit_set(j));
                        assert(!old_self.bitmap@.set_bits.contains(j));
                        assert(0 <= j < old_self.bitmap@.num_bits);
                    }
                    assert(frames.subset_of(old_self@.free_frames));
                }
                Ok(frame_address)
            },
            Err(error) => {
                #[cfg(not(verus_keep_ghost))]
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
        proof_decl! {
            let ghost pa: int = frame@;
            let ghost old_self = *self;
        }
        let raw: usize = frame.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        proof! {
            assert(mem::FRAME_SIZE as int == spec_page_size());
            assert(raw as int == pa);
            assert(pa % spec_page_size() == 0);
            assert(frame_number as int == pa / spec_page_size());
            lemma_frame_facts(self, pa, frame_number as int);
        }

        if frame_number >= self.refcount.len() {
            proof! {
                assert(self.refcount@.len() >= self.bitmap@.num_bits);
                assert(!self@.allocated_frames.contains(pa));
            }
            let reason: &str = "frame number out of bounds";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Reject double-frees: the frame must currently have at least one owner.
        if self.refcount[frame_number] == 0 {
            proof! {
                let fnn = frame_number as int;
                if fnn < self.bitmap@.num_bits {
                    assert(self.bitmap@.set_bits.contains(fnn) <==> self.refcount@[fnn] > 0);
                }
                assert(!self.bitmap@.set_bits.contains(fnn));
                assert(!self@.allocated_frames.contains(pa));
            }
            let reason: &str = "frame is already free";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        proof! {
            let fnn = frame_number as int;
            // The frame is allocated: its slot is non-zero, so its bit is set.
            assert(self.refcount@[fnn] > 0);
            assert(fnn < self.bitmap@.num_bits);
            assert(self.bitmap@.set_bits.contains(fnn));
            assert(self@.allocated_frames.contains(pa));
            assert(self@.refcounts.contains_key(pa));
            assert(self@.refcounts[pa] == self.refcount@[fnn]);
        }

        self.refcount[frame_number] -= 1;

        // Only release the bit in the bitmap when the last owner releases the frame.
        if self.refcount[frame_number] == 0 {
            proof_decl! { let ghost mid_self = *self; }
            match self.bitmap.clear(frame_number) {
                Ok(()) => {
                    proof! {
                        let fnn = frame_number as int;
                        assert(old_self.refcount@[fnn] == 1);
                        assert(self.refcount@ == old_self.refcount@.update(fnn, 0));
                        assert(self.bitmap@.set_bits == old_self.bitmap@.set_bits.remove(fnn));
                        assert(self.bitmap@.num_bits == old_self.bitmap@.num_bits);
                        lemma_refcount_clear(&old_self, self, fnn, pa);
                        lemma_internal_inv_implies_wf(self);
                        assert(old_self@.refcounts[pa] == 1);
                    }
                    Ok(())
                },
                Err(error) => {
                    proof! {
                        // The bit is set and in range, so clear cannot fail here.
                        let fnn = frame_number as int;
                        assert(fnn < mid_self.bitmap@.num_bits);
                        assert(mid_self.bitmap@.set_bits.contains(fnn));
                        assert(mid_self.bitmap@.is_bit_set(fnn));
                        assert(false);
                    }
                    #[cfg(not(verus_keep_ghost))]
                    error!("{error:?} (frame={frame:?})");
                    Err(error)
                },
            }
        } else {
            proof! {
                let fnn = frame_number as int;
                let new_val = self.refcount@[fnn];
                assert(old_self.refcount@[fnn] > 1);
                assert(self.refcount@ == old_self.refcount@.update(fnn, new_val));
                assert(new_val as int == old_self.refcount@[fnn] - 1);
                assert(new_val > 0);
                lemma_refcount_bump(&old_self, self, fnn, new_val, pa);
                lemma_internal_inv_implies_wf(self);
                assert(old_self@.refcounts[pa] == old_self.refcount@[fnn]);
                assert(new_val as int == old_self@.refcounts[pa] - 1);
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
        proof_decl! {
            let ghost pa: int = frame@;
            let ghost old_self = *self;
        }
        let raw: usize = frame.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        proof! {
            assert(mem::FRAME_SIZE as int == spec_page_size());
            assert(raw as int == pa);
            assert(pa % spec_page_size() == 0);
            assert(frame_number as int == pa / spec_page_size());
            lemma_frame_facts(self, pa, frame_number as int);
        }

        if frame_number >= self.refcount.len() {
            proof! {
                assert(self.refcount@.len() >= self.bitmap@.num_bits);
            }
            let reason: &str = "frame number out of bounds";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // The frame must currently have at least one owner. Sharing an unallocated
        // frame is a logic error.
        if self.refcount[frame_number] == 0 {
            proof! {
                let fnn = frame_number as int;
                if fnn < self.bitmap@.num_bits {
                    assert(self.bitmap@.set_bits.contains(fnn) <==> self.refcount@[fnn] > 0);
                }
                assert(!self.bitmap@.set_bits.contains(fnn));
            }
            let reason: &str = "cannot share an unallocated frame";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let new_count: u8 = match self.refcount[frame_number].checked_add(1) {
            Some(n) => n,
            None => {
                proof! {
                    // The slot is saturated at 255, so the frame is allocated with refcount 255.
                    let fnn = frame_number as int;
                    assert(self.refcount@[fnn] == 255);
                    assert(fnn < self.bitmap@.num_bits);
                    assert(self.bitmap@.set_bits.contains(fnn));
                    assert(self@.refcounts[pa] == self.refcount@[fnn]);
                }
                let reason: &str = "frame reference count overflow";
                #[cfg(not(verus_keep_ghost))]
                error!("{reason} (frame={frame:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };
        self.refcount[frame_number] = new_count;

        proof! {
            let fnn = frame_number as int;
            // The frame is allocated: its slot was non-zero, so its index is bitmap-managed and set.
            assert(old_self.refcount@[fnn] > 0);
            assert(fnn < old_self.bitmap@.num_bits);
            assert(old_self.bitmap@.set_bits.contains(fnn));
            assert(self.bitmap == old_self.bitmap);
            assert(self.refcount@ == old_self.refcount@.update(fnn, new_count));
            assert(new_count as int == old_self.refcount@[fnn] + 1);
            // The bump keeps the bit set and the slot positive, preserving `internal_inv` and
            // changing the view only by incrementing `pa`'s refcount.
            lemma_refcount_bump(&old_self, self, fnn, new_count, pa);
            lemma_internal_inv_implies_wf(self);
            assert(new_count as int == old_self@.refcounts[pa] + 1);
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
        proof_decl! {
            let ghost pa: int = frame@;
        }
        proof! {
            // Every `FrameAddress` is page-aligned by construction (its type invariant).
            use_type_invariant(&frame);
        }
        let raw: usize = frame.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        proof! {
            assert(mem::FRAME_SIZE as int == spec_page_size());
            assert(raw as int == pa);
            assert(pa % spec_page_size() == 0);
            assert(frame_number as int == pa / spec_page_size());
            lemma_frame_facts(self, pa, frame_number as int);
        }

        if frame_number >= self.refcount.len() {
            proof! {
                // refcount slice covers at least the bitmap range, so the index is out of the
                // bitmap too — the frame is not allocated.
                assert(self.refcount@.len() >= self.bitmap@.num_bits);
            }
            let reason: &str = "frame number out of bounds";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        if self.refcount[frame_number] == 0 {
            proof! {
                // A zero slot within the slice corresponds to a clear (or out-of-range) bit, so the
                // frame is not allocated.
                let fnn = frame_number as int;
                if fnn < self.bitmap@.num_bits {
                    assert(self.bitmap@.set_bits.contains(fnn) <==> self.refcount@[fnn] > 0);
                }
                assert(!self.bitmap@.set_bits.contains(fnn));
            }
            let reason: &str = "frame is not allocated";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        proof! {
            // The slot is non-zero and within the slice, so its frame index is bitmap-managed and
            // its bit is set; hence the frame is allocated and its refcount equals the slot.
            let fnn = frame_number as int;
            assert(self.refcount@[fnn] > 0);
            if fnn >= self.bitmap@.num_bits {
                assert(self.refcount@[fnn] == 0);
            }
            assert(self.bitmap@.set_bits.contains(fnn));
            assert(self@.refcounts[pa] == self.refcount@[fnn]);
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
        proof_decl! {
            let ghost pa: int = phys_addr@;
            let ghost old_self = *self;
        }
        let raw: usize = phys_addr.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        proof! {
            assert(mem::FRAME_SIZE as int == spec_page_size());
            assert(raw as int == pa);
            assert(pa % spec_page_size() == 0);
            assert(frame_number as int == pa / spec_page_size());
            lemma_frame_facts(self, pa, frame_number as int);
        }
        match self.bitmap.set(frame_number) {
            Ok(()) => {
                #[cfg(not(verus_keep_ghost))]
                debug_assert_eq!(self.refcount[frame_number], 0);
                proof! {
                    let fnn = frame_number as int;
                    // `set` succeeded, so the index was in range and previously clear: the frame was
                    // free and its refcount slot was zero.
                    assert(fnn < self.bitmap@.num_bits);
                    assert(!old_self.bitmap@.set_bits.contains(fnn));
                    assert(old_self@.free_frames.contains(pa));
                    assert(old_self.refcount@[fnn] == 0);
                }
                self.refcount[frame_number] = 1;
                proof! {
                    let fnn = frame_number as int;
                    assert(self.bitmap@.set_bits == old_self.bitmap@.set_bits.insert(fnn));
                    assert(self.bitmap@.num_bits == old_self.bitmap@.num_bits);
                    assert(self.refcount@ == old_self.refcount@.update(fnn, 1));
                    lemma_refcount_book(&old_self, self, fnn, pa);
                    lemma_internal_inv_implies_wf(self);
                }
                Ok(())
            },
            Err(error) => {
                proof! {
                    // `set` failed: the index was out of range or the bit was already set; either
                    // way the frame is not free, and the allocator state is unchanged.
                    assert(!self@.free_frames.contains(pa));
                }
                #[cfg(not(verus_keep_ghost))]
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
            ret <==> (
                self@.allocated_frames.contains(phys_addr@)
                || self@.free_frames.contains(phys_addr@)
            ),
    )]
    fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool {
        proof_decl! {
            let ghost pa: int = phys_addr@;
        }
        // Compute the frame index by division instead of `into_frame_number()`. Both yield
        // `phys_addr@ / FRAME_SIZE`, but division needs no representable-frame-number precondition
        // and cannot panic on the (reserved) top-of-memory frame.
        let raw: usize = phys_addr.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        let nbits: usize = self.bitmap.number_of_bits();
        proof! {
            assert(mem::FRAME_SIZE as int == spec_page_size());
            assert(raw as int == pa);
            assert(pa % spec_page_size() == 0);
            assert(frame_number as int == pa / spec_page_size());
            lemma_frame_facts(self, pa, frame_number as int);
        }
        frame_number < nbits
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
        proof_decl! {
            let ghost old_self = *self;
            let ghost ps = spec_page_size();
        }
        let start_raw: usize = region.start().into_raw_value();
        let size: usize = region.size();
        // Compute the frame index by division (rather than `into_frame_number`), matching the
        // rest of this module: division needs no representable-frame-number precondition.
        let start_frame_number: usize = start_raw / mem::FRAME_SIZE;
        let count: usize = size / mem::FRAME_SIZE;

        proof! {
            assert(mem::FRAME_SIZE as int == ps);
            assert(start_raw as int == region@.start);
            assert(size as int == region@.size);
            assert(region@.start % ps == 0);
            assert(region@.size % ps == 0);
            assert(region@.size > 0);
            assert(start_frame_number as int == region@.start / ps);
            assert(count as int == region@.size / ps);
            // `region` is page-aligned and non-empty, so the booked range is non-empty.
            assert(count as int >= 1) by (nonlinear_arith)
                requires
                    ps > 0,
                    region@.size > 0,
                    region@.size % ps == 0,
                    count as int == region@.size / ps;
        }
        proof_decl! { let ghost lo: int = start_frame_number as int; }
        proof_decl! { let ghost hi: int = lo + count as int; }
        proof! {
            // `hi == (region@.start + region@.size) / ps` because both endpoints are page-aligned.
            assert(region@.start == ps * lo) by (nonlinear_arith)
                requires
                    ps > 0,
                    region@.start % ps == 0,
                    lo == region@.start / ps;
            assert(region@.size == ps * (count as int)) by (nonlinear_arith)
                requires
                    ps > 0,
                    region@.size % ps == 0,
                    count as int == region@.size / ps;
            assert(region@.start + region@.size == ps * hi) by (nonlinear_arith)
                requires
                    region@.start == ps * lo,
                    region@.size == ps * (count as int),
                    hi == lo + count as int;
            assert((region@.start + region@.size) / ps == hi) by (nonlinear_arith)
                requires ps > 0, region@.start + region@.size == ps * hi;
        }

        let nbits: usize = self.bitmap.number_of_bits();
        // Reject ranges that fall outside the bitmap up front. Some frame in the range would be
        // uncovered, hence not free, so the request must fail with the state left untouched.
        if start_frame_number >= nbits || count > nbits - start_frame_number {
            proof! {
                lemma_view_determined(self, &old_self);
                // Exhibit an in-range frame number `k` that the bitmap does not cover.
                let k: int = if start_frame_number >= nbits { lo } else { nbits as int };
                assert(lo <= k < hi);
                assert(k >= self.bitmap@.num_bits);
                let addr: int = frame_addr_of(k);
                assert(frame_set(lo, hi).contains(addr));
                assert(!old_self@.free_frames.contains(addr)) by {
                    if old_self@.free_frames.contains(addr) {
                        let i = choose|i: int|
                            0 <= i < old_self.bitmap@.num_bits
                                && !(#[trigger] old_self.bitmap@.set_bits.contains(i))
                                && addr == frame_addr_of(i);
                        assert(addr == i * ps);
                        assert(addr == k * ps);
                        assert(i == k) by (nonlinear_arith)
                            requires ps > 0, i * ps == k * ps;
                        assert(false);
                    }
                }
                lemma_map_range_is_frame_set(lo, hi);
                lemma_region_frames_eq(region@.start, region@.size, lo, hi);
                assert(region@.start / spec_page_size() == lo);
                assert((region@.start + region@.size) / spec_page_size() == hi);
                let frames = vstd::set_lib::set_int_range(
                    region@.start / spec_page_size(),
                    (region@.start + region@.size) / spec_page_size())
                    .map(|i: int| i * spec_page_size());
                assert(frames =~= frame_set(lo, hi));
                assert(frames.contains(addr));
                assert(!frames.subset_of(old_self@.free_frames));
            }
            let reason: &str = "frame index not covered by the bitmap";
            #[cfg(not(verus_keep_ghost))]
            error!("{} (region={:?})", reason, region);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        let end_frame_number: usize = start_frame_number + count;
        proof! {
            assert(hi == end_frame_number as int);
            assert(hi <= self.bitmap@.num_bits);
        }

        // Check that every frame in the range is currently free. An already-allocated frame
        // indicates a memory layout bug, so the whole request fails without mutating the state.
        #[cfg_attr(verus_keep_ghost, verus_spec(
            invariant
                lo == start_frame_number as int,
                hi == end_frame_number as int,
                region@.start / spec_page_size() == lo,
                (region@.start + region@.size) / spec_page_size() == hi,
                lo <= index <= hi,
                self.inv(),
                self.bitmap@.num_bits == old_self.bitmap@.num_bits,
                self.bitmap@.set_bits == old_self.bitmap@.set_bits,
                self.refcount@ == old_self.refcount@,
                hi <= self.bitmap@.num_bits,
                forall|j: int| lo <= j < index ==> !self.bitmap@.set_bits.contains(j),
            decreases hi - index,
        ))]
        for index in start_frame_number..end_frame_number {
            match self.bitmap.test(index) {
                Ok(false) => {},
                Ok(true) => {
                    proof! {
                        lemma_view_determined(self, &old_self);
                        let k: int = index as int;
                        assert(lo <= k < hi);
                        assert(self.bitmap@.set_bits.contains(k));
                        let addr: int = frame_addr_of(k);
                        assert(frame_set(lo, hi).contains(addr));
                        assert(!old_self@.free_frames.contains(addr)) by {
                            if old_self@.free_frames.contains(addr) {
                                let i = choose|i: int|
                                    0 <= i < old_self.bitmap@.num_bits
                                        && !(#[trigger] old_self.bitmap@.set_bits.contains(i))
                                        && addr == frame_addr_of(i);
                                assert(addr == i * ps);
                                assert(addr == k * ps);
                                assert(i == k) by (nonlinear_arith)
                                    requires ps > 0, i * ps == k * ps;
                                assert(false);
                            }
                        }
                        lemma_map_range_is_frame_set(lo, hi);
                        lemma_region_frames_eq(region@.start, region@.size, lo, hi);
                        assert(region@.start / spec_page_size() == lo);
                        assert((region@.start + region@.size) / spec_page_size() == hi);
                        let frames = vstd::set_lib::set_int_range(
                            region@.start / spec_page_size(),
                            (region@.start + region@.size) / spec_page_size())
                            .map(|i: int| i * spec_page_size());
                        assert(frames =~= frame_set(lo, hi));
                        assert(frames.contains(addr));
                        assert(!frames.subset_of(old_self@.free_frames));
                    }
                    let conflicting_addr: usize = index * mem::FRAME_SIZE;
                    let region_start: usize = region.start().into_raw_value();
                    let region_end: usize = region_start.saturating_add(region.size());
                    let reason: &str = "frame is already allocated";
                    #[cfg(not(verus_keep_ghost))]
                    error!(
                        "{} (frame={:#010x}, region_start={:#010x}, region_end={:#010x})",
                        reason, conflicting_addr, region_start, region_end
                    );
                    return Err(Error::new(ErrorCode::OutOfMemory, reason));
                },
                Err(err) => {
                    proof! {
                        // `test` only fails when the index is out of range, but coverage above
                        // guarantees `index < num_bits`; this branch is unreachable.
                        assert(index < self.bitmap@.num_bits);
                        assert(false);
                    }
                    return Err(err);
                },
            }
        }
        proof! {
            // Coverage succeeded: the whole range was free in `old_self`.
            assert(self.bitmap@.set_bits == old_self.bitmap@.set_bits);
            assert forall|j: int| lo <= j < hi implies
                !old_self.bitmap@.set_bits.contains(j) by {}
        }

        // Book every frame in the range.
        #[cfg_attr(verus_keep_ghost, verus_spec(
            invariant
                lo == start_frame_number as int,
                hi == end_frame_number as int,
                region@.start / spec_page_size() == lo,
                (region@.start + region@.size) / spec_page_size() == hi,
                lo <= index <= hi,
                self.bitmap.inv(),
                self.bitmap@.num_bits == old_self.bitmap@.num_bits,
                self.refcount@.len() == old_self.refcount@.len(),
                self.refcount@.len() >= self.bitmap@.num_bits,
                hi <= self.bitmap@.num_bits,
                self.bitmap@.set_bits == old_self.bitmap@.set_bits.union(
                    ::bitmap::BitmapView::range_set(lo, index as int)),
                forall|j: int| lo <= j < hi ==> !old_self.bitmap@.set_bits.contains(j),
                forall|j: int| lo <= j < index ==> self.refcount@[j] == 1,
                forall|k: int| 0 <= k < self.refcount@.len() && !(lo <= k < index)
                    ==> self.refcount@[k] == old_self.refcount@[k],
            decreases hi - index,
        ))]
        for index in start_frame_number..end_frame_number {
            proof! {
                // The current bit is unset, so `set` will succeed.
                assert(!::bitmap::BitmapView::range_set(lo, index as int).contains(index as int));
                assert(!self.bitmap@.set_bits.contains(index as int));
                assert(index < self.bitmap@.num_bits);
            }
            proof_decl! { let ghost pre_set = *self; }
            if let Err(error) = self.bitmap.set(index) {
                proof! {
                    // `set` only fails when the bit is out of range or already set, both excluded.
                    assert(false);
                }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
            proof! {
                // `set` inserted `index`; combined with the prefix this extends the booked range.
                assert(self.bitmap@.set_bits == pre_set.bitmap@.set_bits.insert(index as int));
                assert(self.bitmap@.set_bits
                    == old_self.bitmap@.set_bits.union(
                        ::bitmap::BitmapView::range_set(lo, index as int + 1))) by {
                    assert(::bitmap::BitmapView::range_set(lo, index as int).insert(index as int)
                        =~= ::bitmap::BitmapView::range_set(lo, index as int + 1));
                }
            }
            #[cfg(not(verus_keep_ghost))]
            debug_assert_eq!(self.refcount[index], 0);
            proof_decl! { let ghost pre_ref = *self; }
            self.refcount[index] = 1;
            proof! {
                assert(self.refcount@ == pre_ref.refcount@.update(index as int, 1));
                assert(self.bitmap == pre_ref.bitmap);
            }
        }

        proof! {
            assert(self.bitmap@.set_bits == old_self.bitmap@.set_bits.union(
                ::bitmap::BitmapView::range_set(lo, hi)));
            assert forall|j: int| lo <= j < hi implies self.refcount@[j] == 1 by {}
            assert forall|k: int| 0 <= k < self.refcount@.len() && !(lo <= k < hi) implies
                self.refcount@[k] == old_self.refcount@[k] by {}
            assert(region@.start / spec_page_size() == lo);
            assert((region@.start + region@.size) / spec_page_size() == hi);
            // Establish `internal_inv` and the contract's `frames`-based view transition.
            lemma_alloc_range_view(&old_self, self, region@.start, region@.size, lo, hi);
            lemma_internal_inv_implies_wf(self);
            let frames = vstd::set_lib::set_int_range(
                region@.start / spec_page_size(),
                (region@.start + region@.size) / spec_page_size())
                .map(|i: int| i * spec_page_size());
            assert(self@ == FrameAllocView {
                allocated_frames: old_self@.allocated_frames.union(frames),
                free_frames: old_self@.free_frames.difference(frames),
                refcounts: old_self@.refcounts.union_prefer_right(
                    Map::new(|addr: int| frames.contains(addr), |addr: int| 1int)),
            });
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
// Skip/exclude target (see `verus-ai-logs/tcb-allowed.md`): initializes the `static mut`
// singleton and the BSS-backed refcount storage. `external_body` because it materializes
// `&'static mut REFCOUNT_STORAGE` and writes the `MaybeUninit` singleton — raw-memory ops Verus
// cannot verify. Callers rely on it establishing `phys_view().initialized` (via
// `lemma_frame_initialized`) before any other free function runs.
#[verus_verify(external_body)]
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
// Dependency contract for the manager layer: thin singleton wrapper around `Inner::alloc`.
// `external_body` until the `frame` free-function layer is verified; the manager bridges the
// returned address into its own abstract frame partition via a proof lemma.
#[verus_verify(external_body)]
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
// Dependency contract: thin singleton wrapper around `Inner::alloc_contiguous`. The base
// address is page-aligned on success. The address-space range bound (`base@ + count*PS <=
// usize::MAX`) is the fact the manager's per-frame index arithmetic relies upon; it follows
// from `Inner::alloc_contiguous`'s frame-set postcondition plus the allocator invariant
// (bridged in the proving phase).
#[verus_verify(external_body)]
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
// Dependency contract: reports the size of the free partition of the global frame allocator.
// The bitmap-level count (`number_of_bits() - usage()`) equals the abstract `free_count()`
// (`free_frames.len()`): `instance()` pins `inner@ == phys_view().frames`, and
// `lemma_free_count_eq` discharges `free_frames.len() == num_bits - usage()`.
#[verus_spec(result =>
    ensures
        result as nat == crate::mm::phys::phys_view().frames.free_count(),
)]
pub(super) fn free_count() -> usize {
    let inner = instance();
    proof! {
        lemma_free_count_eq(inner);
    }
    inner.bitmap.number_of_bits() - inner.bitmap.usage()
}

/// Free a frame previously returned by [`alloc`].
// Dependency contract: best-effort release of a frame. Callers (the manager's error-cleanup
// paths and the `Drop` impls) ignore the outcome, so no precondition is imposed and no abstract
// postcondition is promised. `opens_invariants none`/`no_unwind` so it is callable from
// `UserFrame::drop`/`KernelFrame::drop`. The underlying `Inner::free` precondition (`frame.inv()`)
// is discharged in the proving phase from the `FrameAddress` type invariant.
#[verus_verify(external_body)]
#[verus_spec(result =>
    ensures
        true,
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
// Dependency contract: pure coverage query over the global frame partition. `true` iff the
// allocator tracks the frame (allocated or free), i.e. `phys_view().frames.covers(phys_addr@)`.
// Used by the MMIO boot path to skip frames above RAM that the bitmap does not cover.
#[verus_spec(ret =>
    requires
        phys_addr.inv(),
    ensures
        ret <==> crate::mm::phys::phys_view().frames.covers(phys_addr@),
)]
pub(super) fn is_covered(phys_addr: PageAligned<PhysicalAddress>) -> bool {
    instance().is_covered(phys_addr)
}

/// Reserve a frame so [`alloc`] will skip it.
// Dependency contract: singleton wrapper around `Inner::book`. Reserves a covered, previously
// free frame (refcount becomes 1) so `alloc` never hands it out. The per-frame reservation is
// recorded in the global partition; the booking transition lives in `Inner::book` and is bridged
// to `phys_view().frames` in the proving phase. The boot caller (`book_mmio_regions`) re-derives
// the region-level booking facts via its own lemmas.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_addr.inv(),
    ensures
        match result {
            Ok(()) => crate::mm::phys::phys_view().frames.reserved(phys_addr@),
            Err(_) => !crate::mm::phys::phys_view().frames.free_frames.contains(phys_addr@),
        },
)]
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
    instance().book(phys_addr)
}

/// Book every frame in the given physical memory region.
// Dependency contract: singleton wrapper around `Inner::alloc_range`. On success every frame in
// the region (which must all be free) is reserved with refcount 1. The region-level transition
// lives in `Inner::alloc_range`; the boot caller (`book_physical_memory_regions`) re-derives the
// region-set booking facts via its own lemmas.
#[verus_verify(external_body)]
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
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error> {
    instance().alloc_range(region)
}

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
pub(super) fn share(frame: FrameAddress) -> Result<(), Error> {
    instance().share(frame)
}

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
pub(super) fn refcount(frame: FrameAddress) -> Result<u8, Error> {
    instance().refcount(frame)
}
