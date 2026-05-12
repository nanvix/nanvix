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
// Conversion Wrappers (external-bottom trust boundary)
//
// Thin wrappers that encapsulate the FrameNumber / FrameAddress conversion chain.
// Verus cannot express assume_specification on generic trait methods (Deref) and
// cannot call exec functions in spec mode on external types without View. These
// wrappers isolate the trust boundary to the conversion logic only.
//==================================================================================================

/// Convert a FrameAddress to its bitmap index (frame number as usize).
// VERUS REWRITE: wraps frame.into_frame_number().into_raw_value()
#[verus_verify(external_body)]
#[verus_spec(ret =>
    requires self_.inv(),
    ensures ret as int == self_@ / spec_page_size(),
)]
fn frame_addr_to_bitmap_index(self_: FrameAddress) -> usize {
    self_.into_frame_number().into_raw_value()
}

/// Convert a bitmap index to a FrameAddress.
// VERUS REWRITE: wraps FrameNumber::from_raw_value + FrameAddress::from_frame_number
#[verus_verify(external_body)]
#[verus_spec(ret =>
    requires
        frame_addr_of(index as int) <= usize::MAX as int,
    ensures
        ret.is_ok(),
        ret matches Ok(fa) ==> {
            &&& fa@ == index as int * spec_page_size()
            &&& fa.inv()
        },
)]
fn bitmap_index_to_frame_addr(index: usize) -> Result<FrameAddress, Error> {
    let frame_number = FrameNumber::from_raw_value(index)
        .ok_or_else(|| Error::new(ErrorCode::OutOfMemory, "frame number is out of bounds"))?;
    FrameAddress::from_frame_number(frame_number)
}

/// Convert a PageAligned<PhysicalAddress> to its bitmap index.
// VERUS REWRITE: wraps phys_addr.into_frame_number().into_raw_value() (via Deref)
#[verus_verify(external_body)]
#[verus_spec(ret =>
    requires self_.inv(),
    ensures ret as int == self_@ / spec_page_size(),
)]
fn page_aligned_pa_to_bitmap_index(self_: PageAligned<PhysicalAddress>) -> usize {
    self_.into_frame_number().into_raw_value()
}

/// Get the start frame number from a TruncatedMemoryRegion.
// VERUS REWRITE: wraps region.start().into_frame_number().into_raw_value()
#[verus_verify(external_body)]
#[verus_spec(ret =>
    requires region.inv(),
    ensures ret as int == region@.start / spec_page_size(),
)]
fn region_start_frame_number(region: &TruncatedMemoryRegion<PhysicalAddress>) -> usize {
    region.start().into_frame_number().into_raw_value()
}

/// Get the raw size from a TruncatedMemoryRegion.
// VERUS REWRITE: wraps region.size()
#[verus_verify(external_body)]
#[verus_spec(ret =>
    ensures ret as int == region@.size,
)]
fn region_size_raw(region: &TruncatedMemoryRegion<PhysicalAddress>) -> usize {
    region.size()
}

/// Get the raw start address from a TruncatedMemoryRegion.
// VERUS REWRITE: wraps region.start().into_raw_value() (via Deref)
#[verus_verify(external_body)]
#[verus_spec(ret =>
    requires region.inv(),
    ensures ret as int == region@.start,
)]
fn region_start_raw(region: &TruncatedMemoryRegion<PhysicalAddress>) -> usize {
    region.start().into_raw_value()
}

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
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            self.inv(),
            match result {
                Ok(frame) => {
                    &&& frame.inv()
                    &&& old(self)@.free_frames.contains(frame@)
                    &&& self@ == old(self)@.spec_alloc(frame@)
                },
                Err(_) => {
                    &&& self@ == old(self)@
                    &&& old(self)@.free_frames.is_empty()
                }
            },
    )]
    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let index: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                proof! {
                    self.lemma_view_unchanged(old(self));
                    old(self).lemma_bitmap_full_means_free_empty();
                    self.lemma_internal_inv_preserved(old(self));
                    self.lemma_inv_implies_wf();
                }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?}");
                return Err(error);
            },
        };
        // VERUS DEVIATION: original had FrameNumber::from_raw_value(index) followed by
        // FrameAddress::from_frame_number(frame_number) with two error-path matches.
        // Verus cannot reason through this chain because `PageAligned<T>::Deref::deref`
        // is `external_body` with no spec, and `assume_specification` cannot match
        // generic signatures. This wrapper encapsulates the same conversion with a spec.
        let result = bitmap_index_to_frame_addr(index);
        proof! {
            let idx = index as int;
            let fa = result.unwrap()@;
            self.lemma_set_bit_updates_view(old(self), idx, fa);
            self.lemma_internal_inv_preserved(old(self));
            self.lemma_inv_implies_wf();
        }
        result
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
            self.inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.allocated_frames.contains(frame@)
                    &&& self@ == old(self)@.spec_free(frame@)
                },
                Err(_) => {
                    &&& self@ == old(self)@
                    &&& !old(self)@.allocated_frames.contains(frame@)
                }
            },
    )]
    fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {
        // VERUS DEVIATION: original was `frame.into_frame_number().into_raw_value()`.
        // Verus cannot reason through the Deref auto-deref chain because
        // `PageAligned<T>::Deref::deref` is `external_body` with no spec, and
        // `assume_specification` cannot match generic method signatures.
        // This wrapper encapsulates the same conversion chain with a spec.
        let frame_number: usize = frame_addr_to_bitmap_index(frame);
        proof! {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(frame@, spec_page_size());
            vstd::arithmetic::mul::lemma_mul_is_commutative(spec_page_size(), frame_number as int);
            assert(frame@ == frame_addr_of(frame_number as int));
        }
        match self.bitmap.clear(frame_number) {
            Ok(()) => {
                proof! {
                    let idx = frame_number as int;
                    let fa = frame@;
                    self.lemma_clear_bit_updates_view(old(self), idx, fa);
                    self.lemma_internal_inv_preserved(old(self));
                    self.lemma_inv_implies_wf();
                }
                Ok(())
            },
            Err(error) => {
                proof! {
                    let idx = frame_number as int;
                    let fa = frame@;
                    self.lemma_view_unchanged(old(self));
                    old(self).lemma_addr_not_allocated(idx, fa);
                    self.lemma_internal_inv_preserved(old(self));
                    self.lemma_inv_implies_wf();
                }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (frame={frame:?})");
                Err(error)
            },
        }
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
            self.inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.free_frames.contains(phys_addr@)
                    &&& self@ == old(self)@.spec_book(phys_addr@)
                },
                Err(_) => {
                    &&& self@ == old(self)@
                    &&& !old(self)@.free_frames.contains(phys_addr@)
                }
            },
    )]
    fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        // VERUS DEVIATION: original was `phys_addr.into_frame_number().into_raw_value()`.
        // Same limitation as Inner::free — generic Deref chain cannot be specified.
        let frame_number: usize = page_aligned_pa_to_bitmap_index(phys_addr);
        proof! {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(phys_addr@, spec_page_size());
            vstd::arithmetic::mul::lemma_mul_is_commutative(spec_page_size(), frame_number as int);
            assert(phys_addr@ == frame_addr_of(frame_number as int));
        }
        match self.bitmap.set(frame_number) {
            Ok(()) => {
                proof! {
                    let idx = frame_number as int;
                    let fa = phys_addr@;
                    self.lemma_set_bit_updates_view(old(self), idx, fa);
                    self.lemma_internal_inv_preserved(old(self));
                    self.lemma_inv_implies_wf();
                }
                Ok(())
            },
            Err(error) => {
                proof! {
                    let idx = frame_number as int;
                    let fa = phys_addr@;
                    self.lemma_view_unchanged(old(self));
                    old(self).lemma_addr_not_free(idx, fa);
                    self.lemma_internal_inv_preserved(old(self));
                    self.lemma_inv_implies_wf();
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
            region@.start + region@.size <= usize::MAX as int,
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
                        &&& self@ == old(self)@.spec_alloc_range(frames)
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
    ) -> Result<(), Error> {
        // VERUS REWRITE: region.start().into_frame_number().into_raw_value() → wrapper
        let start_frame_number: usize = region_start_frame_number(region);
        // VERUS REWRITE: replaced `start + size/FRAME_SIZE - 1` and `..=` (inclusive range)
        // with exclusive upper bound. RangeInclusive<usize> lacks ForLoopGhostIteratorNew.
        // VERUS REWRITE: region.size() → wrapper
        let num_frames: usize = region_size_raw(region) / mem::FRAME_SIZE;

        // Prove: start_frame_number + num_frames does not overflow usize.
        proof! {
            let ps = spec_page_size();
            Self::lemma_frame_quotients_bounded(
                region@.start, region@.size,
                start_frame_number as int, num_frames as int, ps);
        }

        let end_frame_number: usize = start_frame_number + num_frames;

        // Prove: efn == (region@.start + region@.size) / ps,
        // and efn * ps == region@.start + region@.size <= usize::MAX.
        proof! {
            let ps = spec_page_size();
            Self::lemma_end_frame_number_properties(
                region@.start, region@.size,
                start_frame_number as int, num_frames as int, ps);
        }

        // When nightly-performance-optimizations is off, verify that every frame index in the
        // range is covered by the sparse bitmap. SparseBitmap::test() returns Ok(false) for
        // uncovered indices, which would incorrectly appear as "free" and pass the check below,
        // only to fail on set(). With the feature enabled this check is elided because
        // PhysicalAddress construction already guarantees valid physical addresses.
        #[cfg(not(feature = "nightly-performance-optimizations"))]
        #[verus_spec(
            invariant
                self.bitmap.inv()
                && self.bitmap@ =~= old(self).bitmap@
                && self.internal_inv()
                && start_frame_number <= index && index <= end_frame_number
                && start_frame_number as int == region@.start / spec_page_size()
                && end_frame_number as int == (region@.start + region@.size) / spec_page_size()
                && (end_frame_number as int) * spec_page_size() <= usize::MAX as int
                && forall|j: int| start_frame_number as int <= j < index as int ==> self.bitmap@.is_covered(j),
        )]
        for index in start_frame_number..end_frame_number {
            if self.bitmap.find_chunk(index).is_none() {
                proof! {
                    self.lemma_alloc_range_conflict(
                        old(self),
                        start_frame_number as int,
                        end_frame_number as int,
                        index as int,
                    );
                }
                // index < end_frame_number, so index * ps < efn * ps <= usize::MAX
                proof! {
                    let ps = spec_page_size();
                    vstd::arithmetic::mul::lemma_mul_strict_inequality(index as int, end_frame_number as int, ps);
                }
                let uncovered_addr: usize = index * mem::FRAME_SIZE;
                let reason: &str = "frame index not covered by any bitmap chunk";
                #[cfg(not(verus_keep_ghost))]
                error!("{} (frame={:#010x}, region={:?})", reason, uncovered_addr, region);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
        }

        // Check if all frames in the range are free.
        #[verus_spec(
            invariant
                self.bitmap.inv()
                && self.bitmap@ =~= old(self).bitmap@
                && self.internal_inv()
                && start_frame_number <= index && index <= end_frame_number
                && start_frame_number as int == region@.start / spec_page_size()
                && end_frame_number as int == (region@.start + region@.size) / spec_page_size()
                && (end_frame_number as int) * spec_page_size() <= usize::MAX as int
                && (forall|j: int| start_frame_number as int <= j < end_frame_number as int ==> self.bitmap@.is_covered(j))
                && (forall|j: int| start_frame_number as int <= j < index as int ==> !self.bitmap@.set_bits.contains(j)),
        )]
        for index in start_frame_number..end_frame_number {
            match self.bitmap.test(index) {
                Ok(false) => {
                    // Frame is free — nothing to do.
                },
                Ok(true) => {
                    proof! {
                        self.lemma_alloc_range_conflict(
                            old(self),
                            start_frame_number as int,
                            end_frame_number as int,
                            index as int,
                        );
                    }
                    // index < end_frame_number, so index * ps < efn * ps <= usize::MAX
                    proof! {
                        let ps = spec_page_size();
                        vstd::arithmetic::mul::lemma_mul_strict_inequality(index as int, end_frame_number as int, ps);
                    }
                    let conflicting_addr: usize = index * mem::FRAME_SIZE;
                    // VERUS REWRITE: region.start().into_raw_value() → wrapper
                    #[cfg(not(verus_keep_ghost))]
                    let region_start: usize = region_start_raw(region);
                    #[cfg(not(verus_keep_ghost))]
                    let region_end: usize = region_start.saturating_add(region_size_raw(region));
                    let reason: &str = "frame is already allocated";
                    #[cfg(not(verus_keep_ghost))]
                    error!(
                        "{} (frame={:#010x}, region_start={:#010x}, region_end={:#010x})",
                        reason, conflicting_addr, region_start, region_end
                    );
                    return Err(Error::new(ErrorCode::OutOfMemory, reason));
                },
                Err(err) => {
                    proof! { assert(false); }
                    return Err(err);
                },
            }
        }

        // Book all frames in the range.
        #[verus_spec(
            invariant
                self.bitmap.inv()
                && self.bitmap@.chunks =~= old(self).bitmap@.chunks
                && self.bitmap@.set_bits =~= old(self).bitmap@.set_bits.union(
                    vstd::set_lib::set_int_range(start_frame_number as int, index as int)
                )
                && start_frame_number <= index && index <= end_frame_number
                && (forall|j: int| start_frame_number as int <= j < end_frame_number as int ==> old(self).bitmap@.is_covered(j))
                && (forall|j: int| start_frame_number as int <= j < end_frame_number as int ==> !old(self).bitmap@.set_bits.contains(j)),
        )]
        for index in start_frame_number..end_frame_number {
            proof! {
                self.lemma_coverage_transfers(old(self), start_frame_number as int, end_frame_number as int);
                assert(!self.bitmap@.set_bits.contains(index as int));
            }
            if let Err(error) = self.bitmap.set(index) {
                proof! { assert(false); }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
            proof! {
                Self::lemma_range_insert_step(start_frame_number as int, index as int);
            }
        }

        // Post-loop: prove Ok postcondition.
        proof! {
            let sfn = start_frame_number as int;
            let efn = end_frame_number as int;
            self.lemma_alloc_range_updates_view(old(self), sfn, efn);
            self.lemma_internal_inv_preserved(old(self));
            self.lemma_inv_implies_wf();
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
pub(super) unsafe fn init(bitmap: SparseBitmap) -> Result<(), Error> {
    if unlikely(INSTANCE_INIT.load(ORDER)) {
        return Err(Error::new(ErrorCode::InvalidArgument, "frame allocator already initialized"));
    }

    #[cfg(not(verus_keep_ghost))]
    info!(
        "frame allocator: {} frames, {} MB, {} chunk(s)",
        bitmap.capacity(),
        (bitmap.capacity() * mem::FRAME_SIZE) / constants::MEGABYTE,
        bitmap.chunk_count(),
    );

    // SAFETY: single-threaded boot; no other reference to `INSTANCE` exists.
    unsafe { INSTANCE.write(Inner { bitmap }) };
    INSTANCE_INIT.store(true, ORDER);
    Ok(())
}

/// Allocate a frame.
/// Singleton pattern: state transition tracked by Inner::alloc.
#[verus_verify(external_body)]
#[verus_spec(result =>
    ensures
        match result {
            Ok(frame) => frame.inv(),
            // Singleton pattern: cannot express state-preservation without ghost accessor.
            Err(_) => true,
        },
)]
pub(super) fn alloc() -> Result<FrameAddress, Error> {
    instance().alloc()
}

// NOTE: free uses verus! syntax because Drop::drop requires `no_unwind`,
// and the attribute-based syntax does not support `no_unwind`.
verus! {
/// Free a frame previously returned by [`alloc`].
#[verifier::external_body]
pub(super) fn free(frame: FrameAddress) -> (result: Result<(), Error>)
    requires
        frame.inv(),
    ensures
        // Singleton pattern: state transition tracked by Inner::free.
        result.is_ok() || result.is_err(),
    opens_invariants none
    no_unwind
{
    instance().free(frame)
}
}

/// Reserve a frame so [`alloc`] will skip it.
/// Singleton pattern: state transition tracked by Inner::book.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        phys_addr.inv(),
    ensures
        // Singleton pattern: cannot express state transition without ghost accessor.
        result.is_ok() || result.is_err(),
)]
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
    instance().book(phys_addr)
}

/// Book every frame in the given physical memory region.
/// Singleton pattern: state transition tracked by Inner::alloc_range.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        region.inv(),
    ensures
        // Singleton pattern: cannot express state transition without ghost accessor.
        result.is_ok() || result.is_err(),
)]
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error> {
    instance().alloc_range(region)
}
