// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Kernel frame pool — module-level singleton.
//!
//! The kernel pool is backed by a [`Bitmap`] and exposed as free functions over a singleton so
//! every in-kernel caller goes through the same state. The public facade types [`Kpool`] and
//! [`KernelFrame`] delegate to the singleton.
//!
//! Access to the kernel pool is synchronized externally and performed by a single thread, so
//! the backing bitmap uses non-atomic operations.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::Bitmap,
    hal::{
        mem::{
            FrameAddress,
            PageAligned,
            PhysicalAddress,
            pa_into_raw,
        },
        platform::is_valid_physical_region,
    },
};
#[cfg(verus_keep_ghost)]
use crate::collections::BitmapView;
#[cfg(verus_keep_ghost)]
use crate::hal::mem::spec_page_size;
use ::alloc::vec::Vec;
use ::arch::mem;
use ::core::{
    hint::unlikely,
    mem::MaybeUninit,
    ops::{
        Deref,
        DerefMut,
    },
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

use ::vstd::prelude::*;

#[cfg(verus_keep_ghost)]
include!("kpool.spec.rs");

#[cfg(verus_keep_ghost)]
include!("kpool.proof.rs");

//==================================================================================================
// Inner
//==================================================================================================

/// Private state of the kernel pool singleton.
#[verus_verify(external_derive)]
struct Inner {
    /// Base address of the kernel pool.
    base: PageAligned<PhysicalAddress>,
    /// Bitmap of free frames.
    bitmap: Bitmap,
}

#[verus_verify]
impl Inner {
    ///
    /// # Description
    ///
    /// Creates a new kernel pool.
    ///
    /// # Return Values
    ///
    /// Upon success, the kernel pool.
    ///
    #[verus_spec(result =>
        requires
            base.inv(),
            bitmap.inv(),
        ensures
            result matches Ok(kpool) ==> {
                &&& kpool.inv()
                &&& kpool@.start == base@
                &&& kpool@.num_pages == bitmap@.num_bits
                &&& kpool@.used_page_indices == Set::<int>::new(|i: int| bitmap@.is_bit_set(i))
            },
    )]
    fn new(base: PageAligned<PhysicalAddress>, bitmap: Bitmap) -> Result<Inner, Error> {
        // Check if bitmap spans across physically-addressable memory.
        let bitmap_capacity: usize = bitmap.number_of_bits();
        let kpool_size: usize = match bitmap_capacity.checked_mul(mem::PAGE_SIZE) {
            Some(size) => size,
            None => {
                let reason: &str = "kernel pool size overflows addressable memory";
                #[cfg(not(verus_keep_ghost))]
                error!("{reason}");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        // VERUS REWRITE: pa_into_raw wrapper needed because Verus cannot resolve generic trait .into_raw_value()
        if !is_valid_physical_region(pa_into_raw(base), kpool_size) {
            let reason: &str = "kernel pool bitmap spans across physically-addressable memory";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        #[cfg(not(verus_keep_ghost))]
        info!("kernel pool: {} frames, {} KB", bitmap_capacity, kpool_size / 1024,);

        proof! {
            // kpool_size == bitmap_capacity * PAGE_SIZE == bitmap@.num_bits * spec_page_size()
            // is_valid_physical_region returned true, so base@ + kpool_size <= usize::MAX + 1
            assert(spec_page_size() > 0);
            assert(bitmap@.num_bits == bitmap_capacity as int);
            assert(kpool_size as int == bitmap_capacity as int * spec_page_size());
            assert(base@ + kpool_size as int <= usize::MAX as int + 1);
            assert(base@ >= 0);
        }

        // VERUS REWRITE: intermediate binding for proof block (pre-approved deviation)
        let inner = Inner { base, bitmap };
        proof! {
            inner.lemma_internal_inv_intro();
        }
        Ok(inner)
    }

    ///
    /// # Description
    ///
    /// Allocates a frame from the kernel pool.
    ///
    /// # Return Values
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
                    let page_index = (frame@ - old(self)@.start) / spec_page_size();
                    &&& frame.inv()
                    &&& 0 <= page_index < old(self)@.num_pages
                    &&& !old(self)@.used_page_indices.contains(page_index)
                    &&& self@ == KpoolView {
                        used_page_indices: old(self)@.used_page_indices.insert(page_index),
                        ..old(self)@
                    }
                },
                Err(_) => {
                    &&& forall|i: int| 0 <= i < old(self)@.num_pages ==> old(self)@.used_page_indices.contains(i)
                    &&& self@ == old(self)@
                },
            },
    )]
    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        proof! {
            self.lemma_internal_inv();
        }
        let index: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?}");
                proof! {
                    // bitmap.alloc() Err => bitmap is full => all pages are used
                    assert(old(self).bitmap@.is_full());
                    // is_full means all bits 0..num_bits are set
                    assert(forall|i: int| 0 <= i < old(self)@.num_pages ==> old(self)@.used_page_indices.contains(i));
                }
                return Err(error);
            },
        };
        proof! {
            // index from bitmap.alloc: 0 <= index < num_bits
            assert(0 <= index < self.bitmap@.num_bits);
            assert(spec_page_size() > 0);
            // Re-establish internal_inv (bitmap mutated but properties preserved)
            self.lemma_internal_inv_intro();

            // Prove index * PAGE_SIZE doesn't overflow using multiplication lemma
            vstd::arithmetic::mul::lemma_mul_strict_inequality(
                index as int,
                self.bitmap@.num_bits,
                spec_page_size(),
            );
            // Now we know: index * spec_page_size() < num_bits * spec_page_size()
            assert(self.base@ + self.bitmap@.num_bits * spec_page_size() <= usize::MAX as int + 1);
            assert(self.base@ >= 0);
            // So index * page_size < num_bits * page_size <= usize::MAX + 1 - base@ <= usize::MAX + 1
            assert((index as int) * spec_page_size() <= usize::MAX as int);

            // Prove base@ + index * PAGE_SIZE doesn't overflow
            assert(self.base@ + (index as int) * spec_page_size() <= usize::MAX as int);

            // Prove page-alignment for FrameAddress::from_raw_value precondition
            assert(self.base@ % spec_page_size() == 0);
        }
        // VERUS REWRITE: pa_into_raw wrapper needed; FrameAddress::from_raw_value equivalent to FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(addr)?)?)
        let addr: usize = pa_into_raw(self.base) + index * mem::PAGE_SIZE;        proof! {
            assert(addr as int == self.base@ + (index as int) * spec_page_size());
            // Prove page alignment: (base@ + index * page_size) % page_size == 0
            vstd::arithmetic::div_mod::lemma_mod_multiples_vanish(
                index as int,
                self.base@,
                spec_page_size(),
            );
            assert(addr as int % spec_page_size() == 0);
        }
        let frame = FrameAddress::from_raw_value(addr)?;
        proof! {
            // frame@ == addr == base@ + index * page_size
            assert(frame@ == addr as int);
            // page_index = (frame@ - old(self)@.start) / page_size = index
            assert(frame@ - old(self)@.start == (index as int) * spec_page_size());
            vstd::arithmetic::div_mod::lemma_div_by_multiple(
                index as int,
                spec_page_size(),
            );
            // Now: (index * page_size) / page_size == index
            let page_index = (frame@ - old(self)@.start) / spec_page_size();
            assert(page_index == index as int);
            // Bitmap alloc: bit index was unset, now set
            assert(!old(self).bitmap@.is_bit_set(index as int));
            assert(!old(self)@.used_page_indices.contains(page_index));
            assert(self.bitmap@.set_bits =~= old(self).bitmap@.set_bits.insert(page_index));
            assert(self@.used_page_indices =~= old(self)@.used_page_indices.insert(page_index));
        }
        Ok(frame)
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of frame addresses from the kernel pool.
    ///
    /// # Parameters
    ///
    /// - `count` - The number of frames to allocate.
    /// - `addrs`: Mutable reference to a pre-allocated vector in which
    ///   to store those frames' addresses.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `addrs` is filled with `count`
    /// contiguous entries. Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            self.inv(),
            match result {
                Ok(()) => {
                    &&& old(addrs)@.len() == 0
                    &&& count > 0
                    &&& addrs@.len() == count
                    &&& forall|which_frame: int| #![trigger addrs@[which_frame]]
                        0 <= which_frame < count ==> {
                            let frame = addrs@[which_frame];
                            let addr = frame@;
                            let page_index = (addr - old(self)@.start) / spec_page_size();
                            &&& frame.inv()
                            &&& 0 <= page_index < old(self)@.num_pages
                            &&& addr == addrs@[0]@ + which_frame * spec_page_size()
                            &&& !old(self)@.used_page_indices.contains(page_index)
                        }
                    &&& {
                        let first_page_index = (addrs@[0]@ - old(self)@.start) / spec_page_size();
                        let new_page_indices = Set::<int>::new(
                            |i: int| first_page_index <= i < first_page_index + count
                        );
                        self@ == KpoolView {
                            used_page_indices: old(self)@.used_page_indices.union(new_page_indices),
                            ..old(self)@
                        }
                    }
                },
                Err(_) => {
                    &&& old(addrs)@.len() > 0 || count == 0 || forall|i: int| !old(self)@.range_free(i, count as int)
                    &&& self@ == old(self)@
                    &&& addrs@ == old(addrs)@
                },
            },
    )]
    fn alloc_range(&mut self, count: usize, addrs: &mut Vec<FrameAddress>) -> Result<(), Error> {
        proof! {
            self.lemma_internal_inv();
        }
        if !addrs.is_empty() {
            let reason: &str = "addrs vector is not empty";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason}");
            proof! {
                assert(old(addrs)@.len() > 0);
            }
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // VERUS REWRITE: guard needed because bitmap.alloc_range requires size > 0
        if count == 0 {
            let reason: &str = "count must be positive";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // VERUS REWRITE: guard needed because bitmap.alloc_range requires size <= num_bits
        let num_pages: usize = self.bitmap.number_of_bits();
        if count > num_pages {
            let reason: &str = "count exceeds pool capacity";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason}");
            proof! {
                // count > num_pages, so range_free(start, count) is vacuously false:
                // range_free requires 0 <= start <= num_pages - count, impossible when count > num_pages
                assert forall|start: int| !old(self)@.range_free(start, count as int) by {};
            }
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let index: usize = match self.bitmap.alloc_range(count) {
            Ok(index) => index,
            Err(error) => {
                proof! {
                    // bitmap.alloc_range Err: no contiguous free range exists
                    assert(!old(self).bitmap@.exists_contiguous_free_range(count as int));
                    assert(count > 0);
                    assert forall|start: int| !old(self)@.range_free(start, count as int) by {
                        if old(self)@.range_free(start, count as int) {
                            assert(0 <= start <= old(self)@.num_pages - count as int);
                            assert(start + count as int <= old(self)@.num_pages);
                            assert(old(self)@.num_pages == old(self).bitmap@.num_bits as int);
                            assert(start + count as int <= old(self).bitmap@.num_bits);
                            // range_free means forall pages in range are not used
                            assert(forall|j: int| start <= j < start + count as int
                                ==> !old(self)@.used_page_indices.contains(j));
                            assert forall|j: int| start <= j < start + count as int
                                implies !old(self).bitmap@.is_bit_set(j) by {};
                            assert(old(self).bitmap@.all_bits_unset_in_range(start, start + count as int));
                            assert(0 <= start);
                            assert(start + count as int <= old(self).bitmap@.num_bits);
                            assert(old(self).bitmap@.has_free_range_at(start, count as int));
                            assert(false);
                        }
                    };
                }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (count={count})");
                return Err(error);
            },
        };

        proof! {
            // index from bitmap.alloc_range: 0 <= index < num_bits, index + count <= num_bits
            assert(0 <= index < self.bitmap@.num_bits);
            assert((index as int) + (count as int) <= self.bitmap@.num_bits);
            assert(spec_page_size() > 0);
            assert(self.base@ >= 0);
            assert(self.base@ + self.bitmap@.num_bits * spec_page_size() <= usize::MAX as int + 1);

            // Prove index * PAGE_SIZE doesn't overflow
            vstd::arithmetic::mul::lemma_mul_strict_inequality(
                index as int,
                self.bitmap@.num_bits,
                spec_page_size(),
            );
            assert((index as int) * spec_page_size() < self.bitmap@.num_bits * spec_page_size());
            assert((index as int) * spec_page_size() <= usize::MAX as int);

            // Prove base@ + index * page_size doesn't overflow
            assert(self.base@ + (index as int) * spec_page_size() <= usize::MAX as int);
        }
        // VERUS REWRITE: pa_into_raw wrapper needed (see Inner::alloc)
        let base_addr: usize = pa_into_raw(self.base) + index * mem::PAGE_SIZE;
        proof! {
            let ghost base_addr_spec = self.base@ + (index as int) * spec_page_size();
            assert(base_addr as int == base_addr_spec);
            // Re-establish internal_inv after bitmap mutation
            // bitmap.alloc_range preserved: inv, num_bits unchanged
            // base unchanged
            self.lemma_internal_inv_intro();
        }
        #[verus_spec(
            invariant
                spec_page_size() > 0,
                self.base@ >= 0,
                self.base@ % spec_page_size() == 0,
                self.base.inv(),
                self.bitmap.inv(),
                self.bitmap@.num_bits == old(self).bitmap@.num_bits,
                self.base@ + self.bitmap@.num_bits * spec_page_size() <= usize::MAX as int + 1,
                self.bitmap@.num_bits < u32::MAX as int,
                (index as int) + (count as int) <= self.bitmap@.num_bits,
                0 <= index,
                base_addr as int == self.base@ + (index as int) * spec_page_size(),
                addrs@.len() == i as int,
                forall|j: int| #![trigger addrs@[j]]
                    0 <= j < i as int ==> {
                        let frame = addrs@[j];
                        &&& frame.inv()
                        &&& frame@ == self.base@ + ((index as int) + j) * spec_page_size()
                    },
        )]
        for i in 0..count
        {
            proof! {
                // Prove i * PAGE_SIZE doesn't overflow
                vstd::arithmetic::mul::lemma_mul_strict_inequality(
                    i as int,
                    count as int,
                    spec_page_size(),
                );
                vstd::arithmetic::mul::lemma_mul_inequality(
                    count as int,
                    self.bitmap@.num_bits,
                    spec_page_size(),
                );
                // i * ps < count * ps <= num_bits * ps
                // base >= 0, so num_bits * ps <= usize::MAX + 1
                // Hence i * ps <= usize::MAX

                // Prove (index + i) * page_size < num_bits * page_size
                vstd::arithmetic::mul::lemma_mul_strict_inequality(
                    (index as int) + (i as int),
                    self.bitmap@.num_bits,
                    spec_page_size(),
                );
                // base + (index+i) * ps < base + num_bits * ps <= usize::MAX + 1
                // So base + (index+i) * ps <= usize::MAX

                // Prove base_addr + i * ps == base + (index + i) * ps
                vstd::arithmetic::mul::lemma_mul_is_distributive_add_other_way(
                    spec_page_size(),
                    index as int,
                    i as int,
                );
                // (index + i) * ps == index * ps + i * ps
                // base + (index + i) * ps == base + index * ps + i * ps == base_addr + i * ps
                assert(base_addr as int + (i as int) * spec_page_size()
                    == self.base@ + ((index as int) + (i as int)) * spec_page_size());
            }
            let addr: usize = base_addr + i * mem::PAGE_SIZE;
            proof! {
                assert(addr as int == self.base@ + ((index as int) + (i as int)) * spec_page_size());
                let ghost a = (index as int) + (i as int);
                let ghost m = spec_page_size();
                let ghost b = self.base@;
                assert(addr as int == b + a * m);
                assert(b + a * m == m * a + b);
                vstd::arithmetic::div_mod::lemma_mod_multiples_vanish(a, b, m);
                // (m * a + b) % m == b % m == 0
                assert(b % m == 0int);
                assert(addr as int % m == 0int);
            }
            // VERUS REWRITE: from_raw_value is equivalent convenience API (see Inner::alloc)
            let frame: FrameAddress = FrameAddress::from_raw_value(addr)?;
            addrs.push(frame);
        }

        proof! {
            // After loop: addrs@.len() == count (from invariant at i == count)
            assert(addrs@.len() == count as int);

            // Re-establish internal_inv for postcondition self.inv()
            self.lemma_internal_inv_intro();

            // Prove page_index for each frame in addrs
            assert forall|which_frame: int| #![trigger addrs@[which_frame]]
                0 <= which_frame < count as int implies ({
                    let frame = addrs@[which_frame];
                    let addr = frame@;
                    let page_index = (addr - old(self)@.start) / spec_page_size();
                    &&& frame.inv()
                    &&& 0 <= page_index < old(self)@.num_pages
                    &&& addr == addrs@[0]@ + which_frame * spec_page_size()
                    &&& !old(self)@.used_page_indices.contains(page_index)
                }) by {
                let frame = addrs@[which_frame];
                // From loop invariant: frame@ == base@ + (index + which_frame) * page_size
                assert(frame@ == self.base@ + ((index as int) + which_frame) * spec_page_size());
                assert(frame.inv());

                // addrs@[0]@ == base@ + index * page_size (from which_frame == 0)
                assert(addrs@[0]@ == self.base@ + (index as int) * spec_page_size());

                // addr == addrs@[0]@ + which_frame * page_size
                // = base@ + index * ps + which_frame * ps
                // = base@ + (index + which_frame) * ps ✓
                vstd::arithmetic::mul::lemma_mul_is_distributive_add(
                    spec_page_size(), index as int, which_frame,
                );

                // page_index = (frame@ - start) / page_size
                //   = ((base@ + (index+wf)*ps) - base@) / ps
                //   = ((index+wf) * ps) / ps
                //   = index + wf
                let page_index = (frame@ - old(self)@.start) / spec_page_size();
                vstd::arithmetic::div_mod::lemma_div_by_multiple(
                    (index as int) + which_frame,
                    spec_page_size(),
                );
                assert(page_index == (index as int) + which_frame);
                assert(0 <= page_index);
                // index + which_frame < index + count <= num_bits == num_pages
                assert(page_index < old(self)@.num_pages);

                // old used_page_indices didn't contain this page:
                // from bitmap.alloc_range: old(self).bitmap@.all_bits_unset_in_range(index, index+count)
                assert(old(self).bitmap@.all_bits_unset_in_range(index as int, index as int + count as int));
                // page_index == index + which_frame, and index <= page_index < index + count
                assert(index as int <= page_index < index as int + count as int);
                // So !old(self).bitmap@.is_bit_set(page_index)
                assert(!old(self).bitmap@.is_bit_set(page_index));
                // is_bit_set(page_index) == set_bits.contains(page_index)
                // set_bits == used_page_indices (from View)
                assert(!old(self)@.used_page_indices.contains(page_index));
            };

            // Prove self@ == KpoolView { used_page_indices: old(self)@.used_page_indices.union(new_page_indices), ..old(self)@ }
            let first_page_index = (addrs@[0]@ - old(self)@.start) / spec_page_size();
            vstd::arithmetic::div_mod::lemma_div_by_multiple(
                index as int,
                spec_page_size(),
            );
            assert(first_page_index == index as int);

            // new_page_indices == Set::new(|i| index <= i < index + count) == BitmapView::range_set(index, index + count)
            let new_page_indices = Set::<int>::new(
                |i: int| first_page_index <= i < first_page_index + count as int,
            );
            assert(new_page_indices =~= BitmapView::range_set(index as int, index as int + count as int));

            // From bitmap.alloc_range ensures:
            // self.bitmap@.set_bits == old(self).bitmap@.set_bits.union(BitmapView::range_set(index, index+count))
            assert(self@.used_page_indices =~= old(self)@.used_page_indices.union(new_page_indices));
            // num_pages unchanged
            assert(self@.num_pages == old(self)@.num_pages);
            // start unchanged
            assert(self@.start == old(self)@.start);
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Frees a previously allocated frame in the kernel pool.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address of the frame to free.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            addr.inv(),
        ensures
            self.inv(),
            ({
                let page_index = (addr@ - old(self)@.start) / spec_page_size();
                let input_valid = {
                    &&& 0 <= page_index < old(self)@.num_pages
                    &&& old(self)@.used_page_indices.contains(page_index)
                };
                match result {
                    Ok(()) => {
                        &&& input_valid
                        &&& self@ == KpoolView {
                              used_page_indices: old(self)@.used_page_indices.remove(page_index),
                              ..old(self)@
                        }
                    },
                    Err(_) => {
                        &&& !input_valid
                        &&& self@ == old(self)@
                    },
                }
            }),
    )]
    fn free(&mut self, addr: FrameAddress) -> Result<(), Error> {
        proof! {
            self.lemma_internal_inv();
        }
        // VERUS REWRITE: guard needed to prevent usize underflow in subtraction
        if addr.into_raw_value() < pa_into_raw(self.base) {
            let reason: &str = "frame address below pool base";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason}");
            proof! {
                // addr@ < start, so page_index < 0, hence !input_valid
                let page_index = (addr@ - old(self)@.start) / spec_page_size();
                let a: int = addr@ - old(self)@.start;
                let d: int = spec_page_size();
                assert(a < 0);
                assert(d > 0);
                vstd::arithmetic::div_mod::lemma_fundamental_div_mod(a, d);
                let q: int = a / d;
                let r: int = a % d;
                assert(a == d * q + r);
                assert(0 <= r);
                assert(d * q == a - r);
                assert(d * q < 0);
                if q >= 0 {
                    vstd::arithmetic::mul::lemma_mul_inequality(0, q, d);
                    assert(false);
                }
                assert(page_index < 0);
            }
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }
        // Now addr@ >= self.base@ == old(self)@.start
        // VERUS REWRITE: pa_into_raw wrapper needed (see Inner::alloc)
        let index: usize = (addr.into_raw_value() - pa_into_raw(self.base)) / mem::PAGE_SIZE;
        proof! {
            assert(addr@ >= old(self)@.start);
            assert(self.base@ == old(self)@.start);
            // Relate exec index to spec page_index
            assert(index as int == (addr@ - old(self)@.start) / spec_page_size());
        }
        match self.bitmap.clear(index) {
            Ok(()) => {
                proof! {
                    // clear Ok: index < num_bits
                    let page_index = (addr@ - old(self)@.start) / spec_page_size();
                    assert(index as int == page_index);
                    assert(0 <= page_index < old(self)@.num_pages);
                    // Prove old(self)@.used_page_indices.contains(page_index):
                    // From clear's spec, usage decreased by 1 and set_bits = old.remove(index).
                    // If index wasn't in old set_bits, remove would be a no-op,
                    // and usage wouldn't decrease — contradiction.
                    assert(self.bitmap@.usage() == old(self).bitmap@.usage() - 1);
                    assert(self.bitmap@.set_bits =~= old(self).bitmap@.set_bits.remove(index as int));
                    old(self).bitmap@.lemma_set_bits_finite();
                    if !old(self).bitmap@.set_bits.contains(index as int) {
                        assert(old(self).bitmap@.set_bits.remove(index as int) =~= old(self).bitmap@.set_bits);
                    }
                    assert(old(self)@.used_page_indices.contains(page_index));
                }
                Ok(())
            },
            Err(error) => {
                proof! {
                    let page_index = (addr@ - old(self)@.start) / spec_page_size();
                    assert(index as int == page_index);
                    // clear Err: index >= num_bits or bit not set => !input_valid
                }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (addr={addr:?})");
                Err(error)
            },
        }
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

/// Whether the kernel pool has been initialized.
static INSTANCE_INIT: AtomicBool = AtomicBool::new(false);

///
/// # Description
///
/// Returns a mutable reference to the initialized singleton.
///
/// # Return Values
///
/// A mutable reference to the kernel pool singleton.
///
fn instance() -> &'static mut Inner {
    if unlikely(!INSTANCE_INIT.load(ORDER)) {
        panic!("kernel pool used before init()");
    }

    // SAFETY: `INSTANCE_INIT` is `true`, so `INSTANCE` has been fully written by `init()`.
    // The kernel is single-threaded with interrupts disabled, so no concurrent access is possible.
    unsafe { INSTANCE.assume_init_mut() }
}

//==================================================================================================
// Public Free Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the kernel pool singleton.
///
/// # Parameters
///
/// - `base`: Base address of the kernel pool.
/// - `bitmap`: Bitmap for tracking free pages.
///
/// # Return Values
///
/// Upon success, a [`Kpool`] instance is returned. Upon failure, an error is returned instead.
///
/// # Safety
///
/// Must be called exactly once during boot, before any other function in this module.
///
pub(super) unsafe fn init(
    base: PageAligned<PhysicalAddress>,
    bitmap: Bitmap,
) -> Result<Kpool, Error> {
    if unlikely(INSTANCE_INIT.load(ORDER)) {
        return Err(Error::new(ErrorCode::InvalidArgument, "kernel pool already initialized"));
    }

    trace!("base={base:?}");

    let inner = Inner::new(base, bitmap)?;

    // SAFETY: single-threaded boot; no other reference to `INSTANCE` exists.
    unsafe { INSTANCE.write(inner) };
    INSTANCE_INIT.store(true, ORDER);
    Ok(Kpool { _private: () })
}

///
/// # Description
///
/// Allocates a frame from the kernel pool.
///
/// # Return Values
///
/// Upon success, the address of the allocated frame is returned. Upon failure, an error is
/// returned instead.
///
fn alloc() -> Result<FrameAddress, Error> {
    instance().alloc()
}

///
/// # Description
///
/// Allocates a contiguous range of frames from the kernel pool.
///
/// # Parameters
///
/// - `count`: Number of frames to allocate.
/// - `addrs`: Mutable reference to a pre-allocated vector into which to
///   store those frames' addresses.
///
/// # Return Values
///
/// Upon success, `Ok(())` is returned and `addrs` is filled with `count`
/// contiguous entries. Upon failure, an error is returned instead.
///
fn alloc_range(count: usize, addrs: &mut Vec<FrameAddress>) -> Result<(), Error> {
    instance().alloc_range(count, addrs)
}

///
/// # Description
///
/// Frees a frame previously returned by [`alloc`].
///
/// # Parameters
///
/// - `addr`: Address of the frame to free.
///
/// # Return Values
///
/// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
///
fn free(addr: FrameAddress) -> Result<(), Error> {
    instance().free(addr)
}

//==================================================================================================
// Kernel Frame
//==================================================================================================

/// A type that represents a kernel frame.
#[verus_verify(external_derive)]
#[derive(Debug)]
pub struct KernelFrame {
    /// Frame address.
    base: FrameAddress,
}

#[cfg(verus_keep_ghost)]
verus! {

impl View for KernelFrame
{
    type V = int;

    closed spec fn view(&self) -> int
    {
        self.base@
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
    /// A kernel frame.
    ///
    fn new(base: FrameAddress) -> Self {
        Self { base }
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
    pub fn clear(&mut self) {
        self.deref_mut().fill(0);
    }
}

impl Deref for KernelFrame {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe {
            core::slice::from_raw_parts(self.base.into_raw_value() as *const u8, mem::PAGE_SIZE)
        }
    }
}

impl DerefMut for KernelFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            core::slice::from_raw_parts_mut(self.base.into_raw_value() as *mut u8, mem::PAGE_SIZE)
        }
    }
}

impl Drop for KernelFrame {
    fn drop(&mut self) {
        if let Err(e) = free(self.base) {
            error!("failed to free kernel frame: {:?}", e);
        }
    }
}

//==================================================================================================
// Kernel Pool
//==================================================================================================

///
/// # Description
///
/// Thin facade over the module-level kernel pool singleton. Exists as a distinct type so
/// kernel-frame allocation has its own entry point ([`Kpool::alloc`] returning [`KernelFrame`]).
///
#[derive(Debug)]
pub struct Kpool {
    /// Private field prevents external construction.
    _private: (),
}

impl Kpool {
    ///
    /// # Description
    ///
    /// Allocates a kernel frame from the kernel frame pool.
    ///
    /// # Return Values
    ///
    /// Upon success, a kernel frame is returned. Upon failure, an error is returned instead.
    ///
    pub fn alloc(&mut self) -> Result<KernelFrame, Error> {
        let addr: FrameAddress = alloc()?;
        Ok(KernelFrame::new(addr))
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of kernel frames from the kernel frame pool.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of frames to allocate.
    /// - `frames`: Mutable reference to a pre-allocated vector into which
    ///   to store those frames' addresses. It must be pre-allocated with
    ///   capacity of at least `count`.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `frames` is filled with `count`
    /// contiguous entries. Upon failure, an error is returned instead.
    ///
    pub fn alloc_many(&mut self, count: usize, frames: &mut Vec<KernelFrame>) -> Result<(), Error> {
        // Check if caller-provided vector is not empty.
        if !frames.is_empty() {
            let reason: &str = "frames vector is not empty";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        if frames.capacity() < count {
            let reason: &str = "frames vector has insufficient capacity";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let mut addrs: Vec<FrameAddress> = Vec::with_capacity(count);
        alloc_range(count, &mut addrs)?;
        for addr in addrs {
            frames.push(KernelFrame::new(addr));
        }
        Ok(())
    }
}
