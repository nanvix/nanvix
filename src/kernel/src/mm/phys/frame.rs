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
                    // bitmap unchanged
                    assert(self.bitmap@.set_bits =~= old(self).bitmap@.set_bits);
                    assert(self.bitmap@.chunks =~= old(self).bitmap@.chunks);

                    // self@ == old(self)@ (view unchanged because bitmap unchanged)
                    assert forall|addr: int|
                        self@.allocated_frames.contains(addr)
                        <==> old(self)@.allocated_frames.contains(addr)
                    by {
                        if self@.allocated_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] self.bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(old(self).bitmap@.set_bits.contains(i));
                        }
                        if old(self)@.allocated_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] old(self).bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(self.bitmap@.set_bits.contains(i));
                        }
                    }
                    assert forall|addr: int|
                        self@.free_frames.contains(addr)
                        <==> old(self)@.free_frames.contains(addr)
                    by {
                        if self@.free_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] self.bitmap@.is_covered(i)
                                && !self.bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(old(self).bitmap@.is_covered(i));
                            assert(!old(self).bitmap@.set_bits.contains(i));
                        }
                        if old(self)@.free_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] old(self).bitmap@.is_covered(i)
                                && !old(self).bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(self.bitmap@.is_covered(i));
                            assert(!self.bitmap@.set_bits.contains(i));
                        }
                    }
                    assert(self@.allocated_frames =~= old(self)@.allocated_frames);
                    assert(self@.free_frames =~= old(self)@.free_frames);

                    // old(self)@.free_frames.is_empty()
                    // bitmap.is_full() means all covered bits are set
                    assert forall|addr: int| !old(self)@.free_frames.contains(addr) by {
                        if old(self)@.free_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] old(self).bitmap@.is_covered(i)
                                && !old(self).bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            // is_full means all covered bits are set
                            assert(old(self).bitmap@.is_bit_set(i));
                            // contradiction with !set_bits.contains(i)
                        }
                    }

                    // self.inv()
                    self.lemma_internal_inv_preserved(old(self));
                    self.lemma_inv_implies_wf();
                }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?}");
                return Err(error);
            },
        };
        proof! {
            let ps = spec_page_size();
            let idx = index as int;
            // bitmap.alloc postconditions: covered(idx), !set(idx), set_bits = old_set_bits.insert(idx)
            assert(old(self).bitmap@.is_covered(idx));
            assert(idx >= 0);
            assert(frame_addr_of(idx) <= usize::MAX as int);
        }
        // VERUS DEVIATION: original had FrameNumber::from_raw_value(index) followed by
        // FrameAddress::from_frame_number(frame_number) with two error-path matches.
        // Verus cannot reason through this chain because `PageAligned<T>::Deref::deref`
        // is `external_body` with no spec, and `assume_specification` cannot match
        // generic signatures. This wrapper encapsulates the same conversion with a spec.
        let result = bitmap_index_to_frame_addr(index);
        proof! {
            let idx = index as int;
            let ps = spec_page_size();
            // bitmap_index_to_frame_addr returns Ok(frame) with frame@ = idx * ps
            let frame = result.unwrap();
            let fa = frame@;
            assert(fa == frame_addr_of(idx));

            // 1. old(self)@.free_frames.contains(fa)
            assert(old(self).bitmap@.is_covered(idx));
            assert(!old(self).bitmap@.set_bits.contains(idx));
            assert(old(self)@.free_frames.contains(fa));

            // 2. self@.allocated_frames =~= old(self)@.allocated_frames.insert(fa)
            assert forall|addr: int| self@.allocated_frames.contains(addr) implies
                old(self)@.allocated_frames.contains(addr) || addr == fa
            by {
                let i = choose|i: int|
                    #[trigger] self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                if i == idx {
                    assert(addr == fa);
                } else {
                    assert(old(self).bitmap@.set_bits.contains(i));
                }
            }
            assert forall|addr: int|
                old(self)@.allocated_frames.contains(addr) || addr == fa
                implies self@.allocated_frames.contains(addr)
            by {
                if addr == fa {
                    assert(self.bitmap@.set_bits.contains(idx));
                } else {
                    let i = choose|i: int|
                        #[trigger] old(self).bitmap@.set_bits.contains(i)
                        && addr == frame_addr_of(i);
                    assert(self.bitmap@.set_bits.contains(i));
                }
            }
            assert(self@.allocated_frames =~= old(self)@.allocated_frames.insert(fa));

            // 3. self@.free_frames =~= old(self)@.free_frames.remove(fa)
            assert forall|addr: int| self@.free_frames.contains(addr) implies
                old(self)@.free_frames.contains(addr) && addr != fa
            by {
                let i = choose|i: int|
                    #[trigger] self.bitmap@.is_covered(i)
                    && !self.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                if i == idx {
                    assert(self.bitmap@.set_bits.contains(idx));
                }
                assert(i != idx);
                assert(!old(self).bitmap@.set_bits.contains(i));
                assert(old(self).bitmap@.is_covered(i));
                if addr == fa {
                    vstd::arithmetic::mul::lemma_mul_is_commutative(i, ps);
                    vstd::arithmetic::mul::lemma_mul_is_commutative(idx, ps);
                    vstd::arithmetic::mul::lemma_mul_equality_converse(ps, i, idx);
                }
            }
            assert forall|addr: int|
                old(self)@.free_frames.contains(addr) && addr != fa
                implies self@.free_frames.contains(addr)
            by {
                let i = choose|i: int|
                    #[trigger] old(self).bitmap@.is_covered(i)
                    && !old(self).bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                if i == idx { assert(addr == fa); }
                assert(i != idx);
                assert(!self.bitmap@.set_bits.contains(i));
                assert(self.bitmap@.is_covered(i));
            }
            assert(self@.free_frames =~= old(self)@.free_frames.remove(fa));

            // 4. self.inv()
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
            // Establish: frame@ == frame_addr_of(frame_number)
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(frame@, spec_page_size());
            assert(frame@ == frame_addr_of(frame_number as int));
        }
        match self.bitmap.clear(frame_number) {
            Ok(()) => {
                proof! {
                    let idx = frame_number as int;
                    let fa = frame@;
                    let ps = spec_page_size();

                    // --- 1. old(self)@.allocated_frames.contains(fa) ---
                    // Witness: idx is in old set_bits
                    assert(old(self).bitmap@.set_bits.contains(idx));
                    assert(fa == frame_addr_of(idx));
                    // Trigger the existential in the view definition
                    assert(old(self)@.allocated_frames.contains(fa));

                    // --- 2. self@.allocated_frames =~= old(self)@.allocated_frames.remove(fa) ---
                    assert forall|addr: int| self@.allocated_frames.contains(addr) implies
                        old(self)@.allocated_frames.contains(addr) && addr != fa
                    by {
                        let i = choose|i: int|
                            #[trigger] self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                        // i in new set_bits = old set_bits \ {idx}
                        assert(old(self).bitmap@.set_bits.contains(i));
                        // i != idx (was removed)
                        if addr == fa {
                            // frame_addr_of(i) == frame_addr_of(idx) with i != idx
                            vstd::arithmetic::mul::lemma_mul_is_commutative(i, ps);
                            vstd::arithmetic::mul::lemma_mul_is_commutative(idx, ps);
                            vstd::arithmetic::mul::lemma_mul_equality_converse(ps, i, idx);
                        }
                    }
                    assert forall|addr: int|
                        old(self)@.allocated_frames.contains(addr) && addr != fa
                        implies self@.allocated_frames.contains(addr)
                    by {
                        let i = choose|i: int|
                            #[trigger] old(self).bitmap@.set_bits.contains(i)
                            && addr == frame_addr_of(i);
                        // i != idx because addr != fa = frame_addr_of(idx)
                        if i == idx { assert(addr == fa); }
                        assert(self.bitmap@.set_bits.contains(i));
                    }
                    assert(self@.allocated_frames =~= old(self)@.allocated_frames.remove(fa));

                    // --- 3. self@.free_frames =~= old(self)@.free_frames.insert(fa) ---
                    assert forall|addr: int| self@.free_frames.contains(addr) implies
                        old(self)@.free_frames.contains(addr) || addr == fa
                    by {
                        let i = choose|i: int|
                            #[trigger] self.bitmap@.is_covered(i)
                            && !self.bitmap@.set_bits.contains(i)
                            && addr == frame_addr_of(i);
                        if i == idx {
                            assert(addr == fa);
                        } else {
                            assert(!old(self).bitmap@.set_bits.contains(i));
                            assert(old(self).bitmap@.is_covered(i));
                        }
                    }
                    assert forall|addr: int|
                        old(self)@.free_frames.contains(addr) || addr == fa
                        implies self@.free_frames.contains(addr)
                    by {
                        if addr == fa {
                            assert(self.bitmap@.is_covered(idx));
                            assert(!self.bitmap@.set_bits.contains(idx));
                        } else {
                            let i = choose|i: int|
                                #[trigger] old(self).bitmap@.is_covered(i)
                                && !old(self).bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            if i == idx { assert(addr == fa); }
                            assert(!self.bitmap@.set_bits.contains(i));
                            assert(self.bitmap@.is_covered(i));
                        }
                    }
                    assert(self@.free_frames =~= old(self)@.free_frames.insert(fa));

                    // --- 4. self.inv() ---
                    self.lemma_internal_inv_preserved(old(self));
                    self.lemma_inv_implies_wf();
                }
                Ok(())
            },
            Err(error) => {
                proof! {
                    let idx = frame_number as int;
                    let fa = frame@;
                    let ps = spec_page_size();

                    // bitmap unchanged
                    assert(self.bitmap@.set_bits =~= old(self).bitmap@.set_bits);
                    assert(self.bitmap@.chunks =~= old(self).bitmap@.chunks);

                    // self@ == old(self)@ (view depends only on bitmap state)
                    assert forall|addr: int|
                        self@.allocated_frames.contains(addr)
                        <==> old(self)@.allocated_frames.contains(addr)
                    by {
                        if self@.allocated_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] self.bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(old(self).bitmap@.set_bits.contains(i));
                        }
                        if old(self)@.allocated_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] old(self).bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(self.bitmap@.set_bits.contains(i));
                        }
                    }
                    assert forall|addr: int|
                        self@.free_frames.contains(addr)
                        <==> old(self)@.free_frames.contains(addr)
                    by {
                        if self@.free_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] self.bitmap@.is_covered(i)
                                && !self.bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(old(self).bitmap@.is_covered(i));
                            assert(!old(self).bitmap@.set_bits.contains(i));
                        }
                        if old(self)@.free_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] old(self).bitmap@.is_covered(i)
                                && !old(self).bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(self.bitmap@.is_covered(i));
                            assert(!self.bitmap@.set_bits.contains(i));
                        }
                    }
                    assert(self@.allocated_frames =~= old(self)@.allocated_frames);
                    assert(self@.free_frames =~= old(self)@.free_frames);

                    // !old(self)@.allocated_frames.contains(fa)
                    // bitmap Err: !is_covered(idx) || !is_bit_set(idx)
                    // In either case, idx not in set_bits (from bitmap wf: set_bits ⊆ covered)
                    assert forall|i: int|
                        old(self).bitmap@.set_bits.contains(i) && fa == frame_addr_of(i)
                        implies false
                    by {
                        // By injectivity, the only candidate is i == idx
                        if i != idx {
                            vstd::arithmetic::mul::lemma_mul_is_commutative(i, ps);
                            vstd::arithmetic::mul::lemma_mul_is_commutative(idx, ps);
                            vstd::arithmetic::mul::lemma_mul_equality_converse(ps, i, idx);
                        }
                    }
                    assert(!old(self)@.allocated_frames.contains(fa));

                    // self.inv()
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
        let frame_number: usize = page_aligned_pa_to_bitmap_index(phys_addr);
        proof! {
            let ps = spec_page_size();
            assert(ps > 0);
            assert(phys_addr@ % ps == 0);
            assert(frame_number as int == phys_addr@ / ps);
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(phys_addr@, ps);
            assert(phys_addr@ == ps * (phys_addr@ / ps) + phys_addr@ % ps);
            assert(phys_addr@ == ps * (frame_number as int));
            vstd::arithmetic::mul::lemma_mul_is_commutative(ps, frame_number as int);
            assert(phys_addr@ == frame_addr_of(frame_number as int));
        }
        match self.bitmap.set(frame_number) {
            Ok(()) => {
                proof! {
                    let idx = frame_number as int;
                    let fa = phys_addr@;
                    let ps = spec_page_size();

                    // --- 1. old(self)@.free_frames.contains(fa) ---
                    assert(old(self).bitmap@.is_covered(idx));
                    assert(!old(self).bitmap@.set_bits.contains(idx));
                    assert(fa == frame_addr_of(idx));
                    assert(old(self)@.free_frames.contains(fa));

                    // --- 2. self@.allocated_frames =~= old(self)@.allocated_frames.insert(fa) ---
                    assert forall|addr: int| self@.allocated_frames.contains(addr) implies
                        old(self)@.allocated_frames.contains(addr) || addr == fa
                    by {
                        let i = choose|i: int|
                            #[trigger] self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                        if i == idx {
                            assert(addr == fa);
                        } else {
                            assert(old(self).bitmap@.set_bits.contains(i));
                        }
                    }
                    assert forall|addr: int|
                        old(self)@.allocated_frames.contains(addr) || addr == fa
                        implies self@.allocated_frames.contains(addr)
                    by {
                        if addr == fa {
                            assert(self.bitmap@.set_bits.contains(idx));
                        } else {
                            let i = choose|i: int|
                                #[trigger] old(self).bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(self.bitmap@.set_bits.contains(i));
                        }
                    }
                    assert(self@.allocated_frames =~= old(self)@.allocated_frames.insert(fa));

                    // --- 3. self@.free_frames =~= old(self)@.free_frames.remove(fa) ---
                    assert forall|addr: int| self@.free_frames.contains(addr) implies
                        old(self)@.free_frames.contains(addr) && addr != fa
                    by {
                        let i = choose|i: int|
                            #[trigger] self.bitmap@.is_covered(i)
                            && !self.bitmap@.set_bits.contains(i)
                            && addr == frame_addr_of(i);
                        // i != idx (idx was added to set_bits)
                        assert(!self.bitmap@.set_bits.contains(i));
                        if i == idx {
                            assert(self.bitmap@.set_bits.contains(idx));
                        }
                        assert(i != idx);
                        assert(!old(self).bitmap@.set_bits.contains(i));
                        assert(old(self).bitmap@.is_covered(i));
                        if addr == fa {
                            vstd::arithmetic::mul::lemma_mul_is_commutative(i, ps);
                            vstd::arithmetic::mul::lemma_mul_is_commutative(idx, ps);
                            vstd::arithmetic::mul::lemma_mul_equality_converse(ps, i, idx);
                        }
                    }
                    assert forall|addr: int|
                        old(self)@.free_frames.contains(addr) && addr != fa
                        implies self@.free_frames.contains(addr)
                    by {
                        let i = choose|i: int|
                            #[trigger] old(self).bitmap@.is_covered(i)
                            && !old(self).bitmap@.set_bits.contains(i)
                            && addr == frame_addr_of(i);
                        if i == idx { assert(addr == fa); }
                        assert(i != idx);
                        assert(!self.bitmap@.set_bits.contains(i));
                        assert(self.bitmap@.is_covered(i));
                    }
                    assert(self@.free_frames =~= old(self)@.free_frames.remove(fa));

                    // --- 4. self.inv() ---
                    self.lemma_internal_inv_preserved(old(self));
                    self.lemma_inv_implies_wf();
                }
                Ok(())
            },
            Err(error) => {
                proof! {
                    let idx = frame_number as int;
                    let fa = phys_addr@;
                    let ps = spec_page_size();

                    // bitmap unchanged
                    assert(self.bitmap@.set_bits =~= old(self).bitmap@.set_bits);
                    assert(self.bitmap@.chunks =~= old(self).bitmap@.chunks);

                    // self@ == old(self)@ (same as free Err case)
                    assert forall|addr: int|
                        self@.allocated_frames.contains(addr)
                        <==> old(self)@.allocated_frames.contains(addr)
                    by {
                        if self@.allocated_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] self.bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(old(self).bitmap@.set_bits.contains(i));
                        }
                        if old(self)@.allocated_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] old(self).bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(self.bitmap@.set_bits.contains(i));
                        }
                    }
                    assert forall|addr: int|
                        self@.free_frames.contains(addr)
                        <==> old(self)@.free_frames.contains(addr)
                    by {
                        if self@.free_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] self.bitmap@.is_covered(i)
                                && !self.bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(old(self).bitmap@.is_covered(i));
                            assert(!old(self).bitmap@.set_bits.contains(i));
                        }
                        if old(self)@.free_frames.contains(addr) {
                            let i = choose|i: int|
                                #[trigger] old(self).bitmap@.is_covered(i)
                                && !old(self).bitmap@.set_bits.contains(i)
                                && addr == frame_addr_of(i);
                            assert(self.bitmap@.is_covered(i));
                            assert(!self.bitmap@.set_bits.contains(i));
                        }
                    }
                    assert(self@.allocated_frames =~= old(self)@.allocated_frames);
                    assert(self@.free_frames =~= old(self)@.free_frames);

                    // !old(self)@.free_frames.contains(fa)
                    // bitmap set Err: !is_covered(idx) || is_bit_set(idx)
                    assert forall|i: int|
                        old(self).bitmap@.is_covered(i)
                        && !old(self).bitmap@.set_bits.contains(i)
                        && fa == frame_addr_of(i)
                        implies false
                    by {
                        if i != idx {
                            vstd::arithmetic::mul::lemma_mul_is_commutative(i, ps);
                            vstd::arithmetic::mul::lemma_mul_is_commutative(idx, ps);
                            vstd::arithmetic::mul::lemma_mul_equality_converse(ps, i, idx);
                        }
                        // i == idx: either !is_covered(idx) or is_bit_set(idx)
                        // Both contradict the antecedent
                    }
                    assert(!old(self)@.free_frames.contains(fa));

                    // self.inv()
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
        // sfn <= region@.start and nf <= region@.size (since ps >= 1),
        // and region@.start + region@.size <= usize::MAX (from requires).
        proof! {
            let ps = spec_page_size();
            lemma_fundamental_div_mod(region@.start, ps);
            lemma_fundamental_div_mod(region@.size, ps);
            // region@.start == ps * sfn (since start % ps == 0)
            // region@.size == ps * nf (since size % ps == 0)
            // sfn <= ps * sfn = region@.start (since ps >= 1)
            vstd::arithmetic::mul::lemma_mul_inequality(1, ps, start_frame_number as int);
            vstd::arithmetic::mul::lemma_mul_is_commutative(ps, start_frame_number as int);
            assert((start_frame_number as int) <= (start_frame_number as int) * ps);
            assert((start_frame_number as int) * ps == region@.start);
            assert((start_frame_number as int) <= region@.start);
            // nf <= ps * nf = region@.size
            vstd::arithmetic::mul::lemma_mul_inequality(1, ps, num_frames as int);
            vstd::arithmetic::mul::lemma_mul_is_commutative(ps, num_frames as int);
            assert((num_frames as int) <= (num_frames as int) * ps);
            assert((num_frames as int) * ps == region@.size);
            assert((num_frames as int) <= region@.size);
            // sfn + nf <= start + size <= usize::MAX
            assert((start_frame_number as int) + (num_frames as int) <= usize::MAX as int);
        }

        let end_frame_number: usize = start_frame_number + num_frames;

        // Prove: efn == (region@.start + region@.size) / ps.
        proof! {
            let ps = spec_page_size();
            lemma_fundamental_div_mod(region@.size, ps);
            vstd::arithmetic::mul::lemma_mul_is_commutative(ps, num_frames as int);
            assert((num_frames as int) * ps == region@.size);
            lemma_hoist_over_denominator(region@.start, num_frames as int, ps as nat);
            assert(end_frame_number as int == (region@.start + region@.size) / ps);
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
                && forall|j: int| start_frame_number as int <= j < index as int ==> self.bitmap@.is_covered(j),
        )]
        for index in start_frame_number..end_frame_number {
            if self.bitmap.find_chunk(index).is_none() {
                proof! {
                    let ps = spec_page_size();
                    // Match postcondition variable definitions exactly
                    let pc_sfn: int = region@.start / ps;
                    let pc_efn: int = (region@.start + region@.size) / ps;
                    let pc_fns = vstd::set_lib::set_int_range(pc_sfn, pc_efn);
                    let pc_frames = pc_fns.map(|i: int| i * ps);

                    // Connect exec variables to postcondition variables
                    assert(pc_sfn == start_frame_number as int);
                    assert(pc_efn == end_frame_number as int);

                    // Prove self@ == old(self)@ (bitmap unchanged in read-only loop)
                    assert(self@.allocated_frames =~= old(self)@.allocated_frames);
                    assert(self@.free_frames =~= old(self)@.free_frames);
                    assert(self@ =~= old(self)@);

                    // Prove !frames.subset_of(old@.free_frames)
                    let fa = frame_addr_of(index as int);
                    assert(pc_fns.contains(index as int));
                    assert(pc_frames.contains(fa));
                    if old(self)@.free_frames.contains(fa) {
                        let w = choose|i: int|
                            old(self).bitmap@.is_covered(i)
                            && !old(self).bitmap@.set_bits.contains(i)
                            && fa == frame_addr_of(i);
                        vstd::arithmetic::mul::lemma_mul_is_commutative(index as int, ps);
                        vstd::arithmetic::mul::lemma_mul_is_commutative(w, ps);
                        vstd::arithmetic::mul::lemma_mul_equality_converse(ps, index as int, w);
                        assert(false);
                    }
                    assert(!pc_frames.subset_of(old(self)@.free_frames));
                    self.lemma_inv_implies_wf();
                }
                // BUG FIX: cfg-gate error-reporting multiply to avoid usize overflow
                #[cfg(not(verus_keep_ghost))]
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
                        let ps = spec_page_size();
                        // Match postcondition variable definitions exactly
                        let pc_sfn: int = region@.start / ps;
                        let pc_efn: int = (region@.start + region@.size) / ps;
                        let pc_fns = vstd::set_lib::set_int_range(pc_sfn, pc_efn);
                        let pc_frames = pc_fns.map(|i: int| i * ps);

                        // Connect exec variables to postcondition variables
                        assert(pc_sfn == start_frame_number as int);
                        assert(pc_efn == end_frame_number as int);

                        // Prove self@ == old(self)@ (bitmap unchanged in read-only loops)
                        assert(self@.allocated_frames =~= old(self)@.allocated_frames);
                        assert(self@.free_frames =~= old(self)@.free_frames);
                        assert(self@ =~= old(self)@);

                        // Prove !frames.subset_of(old@.free_frames)
                        let fa = frame_addr_of(index as int);
                        assert(pc_fns.contains(index as int));
                        assert(pc_frames.contains(fa));
                        if old(self)@.free_frames.contains(fa) {
                            let w = choose|i: int|
                                old(self).bitmap@.is_covered(i)
                                && !old(self).bitmap@.set_bits.contains(i)
                                && fa == frame_addr_of(i);
                            vstd::arithmetic::mul::lemma_mul_is_commutative(index as int, ps);
                            vstd::arithmetic::mul::lemma_mul_is_commutative(w, ps);
                            vstd::arithmetic::mul::lemma_mul_equality_converse(ps, index as int, w);
                            assert(false);
                        }
                        assert(!pc_frames.subset_of(old(self)@.free_frames));
                        self.lemma_inv_implies_wf();
                    }
                    // BUG FIX: cfg-gate error-reporting computations to avoid usize overflow
                    #[cfg(not(verus_keep_ghost))]
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
                // Bridge coverage: chunks are equal, so is_covered transfers.
                assert forall|jj: int|
                    start_frame_number as int <= jj < end_frame_number as int
                    implies self.bitmap@.is_covered(jj)
                by {
                    assert(old(self).bitmap@.is_covered(jj));
                };
                assert(self.bitmap@.is_covered(index as int));
                assert(!old(self).bitmap@.set_bits.contains(index as int));
                assert(!vstd::set_lib::set_int_range(start_frame_number as int, index as int).contains(index as int));
                assert(!self.bitmap@.set_bits.contains(index as int));
            }
            if let Err(error) = self.bitmap.set(index) {
                proof! { assert(false); }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
            proof! {
                assert(vstd::set_lib::set_int_range(start_frame_number as int, (index + 1) as int) =~=
                    vstd::set_lib::set_int_range(start_frame_number as int, index as int).insert(index as int));
                assert(self.bitmap@.set_bits =~= old(self).bitmap@.set_bits.union(
                    vstd::set_lib::set_int_range(start_frame_number as int, (index + 1) as int)
                ));
            }
        }

        // Post-loop: prove Ok postcondition.
        proof! {
            let ps = spec_page_size();
            let sfn = start_frame_number as int;
            let efn = end_frame_number as int;
            let frames = vstd::set_lib::set_int_range(sfn, efn).map(|i: int| i * ps);

            // 1. frames ⊆ old@.free_frames
            assert forall|addr: int| frames.contains(addr)
                implies old(self)@.free_frames.contains(addr)
            by {
                let j = choose|j: int|
                    vstd::set_lib::set_int_range(sfn, efn).contains(j) && addr == j * ps;
                assert(sfn <= j && j < efn);
                assert(old(self).bitmap@.is_covered(j));
                assert(!old(self).bitmap@.set_bits.contains(j));
                assert(addr == frame_addr_of(j));
            }

            // 2. allocated_frames = old.allocated ∪ frames
            assert forall|addr: int|
                self@.allocated_frames.contains(addr) <==>
                old(self)@.allocated_frames.union(frames).contains(addr)
            by {
                if self@.allocated_frames.contains(addr) {
                    let i = choose|i: int|
                        self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                    if old(self).bitmap@.set_bits.contains(i) {
                        assert(old(self)@.allocated_frames.contains(addr));
                    } else {
                        assert(vstd::set_lib::set_int_range(sfn, efn).contains(i));
                        assert(frames.contains(addr));
                    }
                }
                if old(self)@.allocated_frames.contains(addr) {
                    let i = choose|i: int|
                        old(self).bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                    assert(self.bitmap@.set_bits.contains(i));
                    assert(self@.allocated_frames.contains(addr));
                }
                if frames.contains(addr) {
                    let j = choose|j: int|
                        vstd::set_lib::set_int_range(sfn, efn).contains(j) && addr == j * ps;
                    assert(self.bitmap@.set_bits.contains(j));
                    assert(addr == frame_addr_of(j));
                    assert(self@.allocated_frames.contains(addr));
                }
            }

            // 3. free_frames = old.free \ frames
            assert forall|addr: int|
                self@.free_frames.contains(addr) <==>
                old(self)@.free_frames.difference(frames).contains(addr)
            by {
                if self@.free_frames.contains(addr) {
                    let i = choose|i: int|
                        self.bitmap@.is_covered(i)
                        && !self.bitmap@.set_bits.contains(i)
                        && addr == frame_addr_of(i);
                    assert(old(self).bitmap@.is_covered(i));
                    assert(!old(self).bitmap@.set_bits.contains(i));
                    assert(old(self)@.free_frames.contains(addr));
                    assert(!vstd::set_lib::set_int_range(sfn, efn).contains(i));
                    if frames.contains(addr) {
                        let j = choose|j: int|
                            vstd::set_lib::set_int_range(sfn, efn).contains(j) && addr == j * ps;
                        vstd::arithmetic::mul::lemma_mul_is_commutative(i, ps);
                        vstd::arithmetic::mul::lemma_mul_is_commutative(j, ps);
                        vstd::arithmetic::mul::lemma_mul_equality_converse(ps, i, j);
                        assert(false);
                    }
                    assert(!frames.contains(addr));
                }
                if old(self)@.free_frames.contains(addr) && !frames.contains(addr) {
                    let i = choose|i: int|
                        old(self).bitmap@.is_covered(i)
                        && !old(self).bitmap@.set_bits.contains(i)
                        && addr == frame_addr_of(i);
                    // Bridge coverage: chunks unchanged
                    assert(old(self).bitmap@.is_covered(i));
                    assert(self.bitmap@.is_covered(i));
                    if vstd::set_lib::set_int_range(sfn, efn).contains(i) {
                        assert(frames.contains(addr));
                        assert(false);
                    }
                    assert(!vstd::set_lib::set_int_range(sfn, efn).contains(i));
                    assert(!self.bitmap@.set_bits.contains(i));
                    assert(self@.free_frames.contains(addr));
                }
            }

            // 4. Prove spec_alloc_range match
            assert(self@.allocated_frames =~= old(self)@.allocated_frames.union(frames));
            assert(self@.free_frames =~= old(self)@.free_frames.difference(frames));
            assert(self@ =~= old(self)@.spec_alloc_range(frames));

            // 5. self.inv()
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
