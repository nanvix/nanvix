verus! {

use super::FrameAllocView;
use crate::hal::mem::spec_page_size;
use vstd::map::*;
use vstd::set::*;

// The kernel targets x86_64, where `usize` is 8 bytes (`MAX_ADDRESS == usize::MAX == 2^64 - 1`).
// Encoding the target word size lets the verifier discharge frame-number representability bounds
// (e.g. that a bitmap index, capped below `u32::MAX`, is always a valid frame number). This is a
// crate-global fact and weakens no specification.
global size_of usize == 8;

/// Helper: convert a bitmap index to a frame (physical) address.
pub open spec fn frame_addr_of(i: int) -> int {
    i * spec_page_size()
}

impl View for Inner {
    type V = FrameAllocView;

    closed spec fn view(&self) -> FrameAllocView {
        FrameAllocView {
            allocated_frames: Set::new(|addr: int|
                exists|i: int|
                    #[trigger] self.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i)
            ),
            free_frames: Set::new(|addr: int|
                exists|i: int| {
                    &&& 0 <= i < self.bitmap@.num_bits
                    &&& !#[trigger] self.bitmap@.set_bits.contains(i)
                    &&& addr == frame_addr_of(i)
                }
            ),
            refcounts: Map::new(
                |addr: int|
                    exists|i: int|
                        #[trigger] self.bitmap@.set_bits.contains(i)
                        && addr == frame_addr_of(i),
                |addr: int| {
                    let i = addr / spec_page_size();
                    self.refcount@[i] as int
                },
            ),
        }
    }
}

impl Inner {
    pub closed spec fn internal_inv(&self) -> bool
    {
        &&& self.bitmap.inv()
        &&& spec_page_size() > 0
        // refcount slice covers all bitmap-managed frames
        &&& self.refcount@.len() >= self.bitmap@.num_bits
        // bitmap bit set iff refcount > 0
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits ==> (
            #[trigger] self.bitmap@.set_bits.contains(i) <==> self.refcount@[i] > 0
        )
        // bitmap bit clear iff refcount == 0
        // NOTE: logically implied by the above when refcount is u8 (>= 0),
        // but kept explicit to help the SMT solver without relying on type bounds.
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits ==> (
            !self.bitmap@.set_bits.contains(i) <==> self.refcount@[i] == 0
        )
        // refcount bounded by u8
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits && self.bitmap@.set_bits.contains(i) ==>
            0 < self.refcount@[i] <= 255
        // Every covered bitmap index yields a non-negative, representable frame address
        &&& forall|i: int| 0 <= i < self.bitmap@.num_bits ==> {
            &&& frame_addr_of(i) >= 0
            &&& frame_addr_of(i) <= usize::MAX as int
        }
        // Tail-zero: refcount slots beyond the bitmap range must be zero
        &&& forall|i: int| self.bitmap@.num_bits <= i < self.refcount@.len() ==>
            self.refcount@[i] == 0
    }
}

// Relates the integer frame index `fn_ = pa / PAGE_SIZE` of a page-aligned address `pa` to the
// abstract view of the allocator. Every method that turns an address parameter into a bitmap index
// uses these equivalences to connect the executable index to the spec-level frame sets/maps.
proof fn lemma_frame_facts(inner: &Inner, pa: int, fn_: int)
    requires
        inner.inv(),
        pa >= 0,
        pa % spec_page_size() == 0,
        fn_ == pa / spec_page_size(),
    ensures
        pa == fn_ * spec_page_size(),
        fn_ >= 0,
        inner@.allocated_frames.contains(pa) <==> (
            0 <= fn_ < inner.bitmap@.num_bits && inner.bitmap@.set_bits.contains(fn_)
        ),
        inner@.refcounts.contains_key(pa) <==> (
            0 <= fn_ < inner.bitmap@.num_bits && inner.bitmap@.set_bits.contains(fn_)
        ),
        inner@.free_frames.contains(pa) <==> (
            0 <= fn_ < inner.bitmap@.num_bits && !inner.bitmap@.set_bits.contains(fn_)
        ),
{
    let ps: int = spec_page_size();
    let nbits: int = inner.bitmap@.num_bits;
    assert(ps > 0);
    assert(inner.bitmap.inv());
    assert(inner.bitmap@.wf());
    // pa == fn_ * ps (since pa is a multiple of ps).
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(pa, ps);
    assert(pa == ps * fn_);
    assert(pa == fn_ * ps) by (nonlinear_arith)
        requires pa == ps * fn_;
    assert(fn_ >= 0) by (nonlinear_arith)
        requires pa >= 0, ps > 0, pa == fn_ * ps;

    // allocated_frames.contains(pa) <==> (fn_ in range && set).
    assert(inner@.allocated_frames.contains(pa) <==> (
        0 <= fn_ < nbits && inner.bitmap@.set_bits.contains(fn_)
    )) by {
        if inner@.allocated_frames.contains(pa) {
            let i = choose|i: int|
                #[trigger] inner.bitmap@.set_bits.contains(i) && pa == frame_addr_of(i);
            assert(inner.bitmap@.set_bits.contains(i) && pa == frame_addr_of(i));
            assert(0 <= i < nbits);
            assert(pa == i * ps);
            assert(i == fn_) by (nonlinear_arith)
                requires ps > 0, i * ps == fn_ * ps;
        }
        if 0 <= fn_ < nbits && inner.bitmap@.set_bits.contains(fn_) {
            assert(pa == frame_addr_of(fn_));
            assert(inner@.allocated_frames.contains(pa));
        }
    };

    // refcounts share the same key predicate as allocated_frames.
    assert(inner@.refcounts.contains_key(pa) <==> inner@.allocated_frames.contains(pa));

    // free_frames.contains(pa) <==> (fn_ in range && clear).
    assert(inner@.free_frames.contains(pa) <==> (
        0 <= fn_ < nbits && !inner.bitmap@.set_bits.contains(fn_)
    )) by {
        if inner@.free_frames.contains(pa) {
            let i = choose|i: int|
                0 <= i < nbits && !(#[trigger] inner.bitmap@.set_bits.contains(i))
                    && pa == frame_addr_of(i);
            assert(0 <= i < nbits && pa == frame_addr_of(i));
            assert(pa == i * ps);
            assert(i == fn_) by (nonlinear_arith)
                requires ps > 0, i * ps == fn_ * ps;
        }
        if 0 <= fn_ < nbits && !inner.bitmap@.set_bits.contains(fn_) {
            assert(pa == frame_addr_of(fn_));
            assert(inner@.free_frames.contains(pa));
        }
    };
}

// `internal_inv` (a statement about the bitmap and the refcount slice) entails the abstract
// well-formedness `wf()` of the allocator view. Transition proofs maintain only the concrete
// `internal_inv` and recover `wf()` for the `inv()` postcondition through this lemma.
proof fn lemma_internal_inv_implies_wf(inner: &Inner)
    requires
        inner.internal_inv(),
    ensures
        inner@.wf(),
{
    let ps: int = spec_page_size();
    let v = inner@;
    let nbits: int = inner.bitmap@.num_bits;
    assert(ps > 0);
    assert(inner.bitmap.inv());
    assert(inner.bitmap@.wf());

    // Page-alignment of allocated/free addresses (each is `i * ps`).
    assert forall|addr: int| v.allocated_frames.contains(addr) implies addr % ps == 0 by {
        let i = choose|i: int|
            #[trigger] inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
        assert(addr == i * ps);
        vstd::arithmetic::div_mod::lemma_mod_multiples_basic(i, ps);
    };
    assert forall|addr: int| v.free_frames.contains(addr) implies addr % ps == 0 by {
        let i = choose|i: int|
            0 <= i < nbits && !(#[trigger] inner.bitmap@.set_bits.contains(i))
                && addr == frame_addr_of(i);
        assert(addr == i * ps);
        vstd::arithmetic::div_mod::lemma_mod_multiples_basic(i, ps);
    };

    // Disjointness of allocated and free frame sets.
    assert forall|addr: int| v.allocated_frames.contains(addr) implies !v.free_frames.contains(addr) by {
        let i = choose|i: int|
            #[trigger] inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
        assert(addr == i * ps);
        if v.free_frames.contains(addr) {
            let j = choose|j: int|
                0 <= j < nbits && !(#[trigger] inner.bitmap@.set_bits.contains(j))
                    && addr == frame_addr_of(j);
            assert(addr == j * ps);
            assert(i == j) by (nonlinear_arith) requires ps > 0, i * ps == j * ps;
            assert(false);
        }
    };

    // Allocated iff refcount entry exists and is positive.
    assert forall|addr: int| #[trigger] v.allocated_frames.contains(addr) <==>
        (v.refcounts.contains_key(addr) && v.refcounts[addr] > 0) by {
        if v.allocated_frames.contains(addr) {
            let i = choose|i: int|
                #[trigger] inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(0 <= i < nbits);
            assert(addr == i * ps);
            assert(addr / ps == i) by (nonlinear_arith) requires addr == i * ps, ps > 0;
            assert(v.refcounts[addr] == inner.refcount@[i]);
            assert(inner.refcount@[i] > 0);
        }
    };

    // Free frames carry no refcount entry.
    assert forall|addr: int| #[trigger] v.free_frames.contains(addr) implies
        !v.refcounts.contains_key(addr) by {
        let i = choose|i: int|
            0 <= i < nbits && !(#[trigger] inner.bitmap@.set_bits.contains(i))
                && addr == frame_addr_of(i);
        assert(addr == i * ps);
        if v.refcounts.contains_key(addr) {
            let j = choose|j: int|
                #[trigger] inner.bitmap@.set_bits.contains(j) && addr == frame_addr_of(j);
            assert(addr == j * ps);
            assert(i == j) by (nonlinear_arith) requires ps > 0, i * ps == j * ps;
            assert(false);
        }
    };

    // Refcount values are within the u8 range.
    assert forall|addr: int| v.refcounts.contains_key(addr) implies
        0 < v.refcounts[addr] <= 255 by {
        let i = choose|i: int|
            #[trigger] inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
        assert(0 <= i < nbits);
        assert(addr == i * ps);
        assert(addr / ps == i) by (nonlinear_arith) requires addr == i * ps, ps > 0;
        assert(v.refcounts[addr] == inner.refcount@[i]);
        assert(inner.bitmap@.set_bits.contains(i));
        assert(0 < inner.refcount@[i] <= 255);
    };
}

// A refcount update at an already-set index `fnn` to a new positive value preserves `internal_inv`
// and changes the view only by setting `pa`'s (= `fnn * ps`) refcount to the new value. Used by
// `share` (increment) and the shared-still-owned case of `free` (decrement that stays positive).
proof fn lemma_refcount_bump(old_inner: &Inner, new_inner: &Inner, fnn: int, new_val: u8, pa: int)
    requires
        old_inner.inv(),
        new_inner.bitmap == old_inner.bitmap,
        new_inner.refcount@ == old_inner.refcount@.update(fnn, new_val),
        0 <= fnn < old_inner.bitmap@.num_bits,
        old_inner.bitmap@.set_bits.contains(fnn),
        new_val > 0,
        pa == fnn * spec_page_size(),
    ensures
        new_inner.internal_inv(),
        new_inner@.allocated_frames == old_inner@.allocated_frames,
        new_inner@.free_frames == old_inner@.free_frames,
        new_inner@.refcounts =~= old_inner@.refcounts.insert(pa, new_val as int),
{
    let ps: int = spec_page_size();
    let nbits: int = old_inner.bitmap@.num_bits;
    assert(ps > 0);
    assert(old_inner.bitmap.inv());
    assert(old_inner.bitmap@.wf());
    assert(new_inner.refcount@.len() == old_inner.refcount@.len());
    assert(new_inner.refcount@[fnn] == new_val);

    // internal_inv(new_inner): only slot `fnn` changed, and it stays set and positive.
    assert(new_inner.internal_inv()) by {
        assert(new_inner.bitmap.inv());
        assert forall|i: int| 0 <= i < nbits implies (
            #[trigger] new_inner.bitmap@.set_bits.contains(i) <==> new_inner.refcount@[i] > 0
        ) by {
            if i == fnn {
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| 0 <= i < nbits implies (
            !(#[trigger] new_inner.bitmap@.set_bits.contains(i)) <==> new_inner.refcount@[i] == 0
        ) by {
            if i == fnn {
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| 0 <= i < nbits && new_inner.bitmap@.set_bits.contains(i) implies
            0 < #[trigger] new_inner.refcount@[i] <= 255 by {
            if i == fnn {
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| nbits <= i < new_inner.refcount@.len() implies
            #[trigger] new_inner.refcount@[i] == 0 by {
            assert(i != fnn);
            assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
        };
    };

    // The bitmap is unchanged, so the allocated/free sets are unchanged.
    assert(new_inner@.allocated_frames =~= old_inner@.allocated_frames);
    assert(new_inner@.free_frames =~= old_inner@.free_frames);

    // The refcount map changes only at key `pa`.
    assert(old_inner@.refcounts.contains_key(pa)) by {
        assert(pa == frame_addr_of(fnn));
    };
    assert(new_inner@.refcounts =~= old_inner@.refcounts.insert(pa, new_val as int)) by {
        assert forall|addr: int|
            #[trigger] new_inner@.refcounts.contains_key(addr) implies
            old_inner@.refcounts.insert(pa, new_val as int).contains_key(addr) by {}
        assert forall|addr: int|
            old_inner@.refcounts.insert(pa, new_val as int).contains_key(addr) implies
            #[trigger] new_inner@.refcounts.contains_key(addr) by {}
        assert forall|addr: int|
            new_inner@.refcounts.contains_key(addr) implies
            #[trigger] new_inner@.refcounts[addr]
                == old_inner@.refcounts.insert(pa, new_val as int)[addr] by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            assert(addr / ps == i) by (nonlinear_arith) requires addr == i * ps, ps > 0;
            if addr == pa {
                assert(i == fnn) by (nonlinear_arith) requires ps > 0, i * ps == fnn * ps, addr == i * ps, pa == fnn * ps, addr == pa;
            } else {
                assert(i != fnn);
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        }
    };
}

// Releasing the last reference at index `fnn` clears its bit and zeroes its slot. This preserves
// `internal_inv` and moves `pa` (= `fnn * ps`) from the allocated set to the free set, dropping its
// refcount entry. Used by the last-reference case of `free`.
proof fn lemma_refcount_clear(old_inner: &Inner, new_inner: &Inner, fnn: int, pa: int)
    requires
        old_inner.inv(),
        0 <= fnn < old_inner.bitmap@.num_bits,
        old_inner.bitmap@.set_bits.contains(fnn),
        old_inner.refcount@[fnn] == 1,
        new_inner.refcount@ == old_inner.refcount@.update(fnn, 0),
        new_inner.bitmap.inv(),
        new_inner.bitmap@.num_bits == old_inner.bitmap@.num_bits,
        new_inner.bitmap@.set_bits == old_inner.bitmap@.set_bits.remove(fnn),
        pa == fnn * spec_page_size(),
    ensures
        new_inner.internal_inv(),
        new_inner@.allocated_frames =~= old_inner@.allocated_frames.remove(pa),
        new_inner@.free_frames =~= old_inner@.free_frames.insert(pa),
        new_inner@.refcounts =~= old_inner@.refcounts.remove(pa),
{
    let ps: int = spec_page_size();
    let nbits: int = old_inner.bitmap@.num_bits;
    assert(ps > 0);
    assert(old_inner.bitmap.inv());
    assert(old_inner.bitmap@.wf());
    assert(new_inner.refcount@.len() == old_inner.refcount@.len());
    assert(new_inner.refcount@[fnn] == 0);
    assert(pa == frame_addr_of(fnn));
    assert forall|i: int| #[trigger] new_inner.bitmap@.set_bits.contains(i) <==> (
        old_inner.bitmap@.set_bits.contains(i) && i != fnn
    ) by {}

    // internal_inv(new_inner): only slot `fnn` changed; it is now clear and zero.
    assert(new_inner.internal_inv()) by {
        assert(new_inner.bitmap.inv());
        assert forall|i: int| 0 <= i < nbits implies (
            #[trigger] new_inner.bitmap@.set_bits.contains(i) <==> new_inner.refcount@[i] > 0
        ) by {
            if i == fnn {
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| 0 <= i < nbits implies (
            !(#[trigger] new_inner.bitmap@.set_bits.contains(i)) <==> new_inner.refcount@[i] == 0
        ) by {
            if i == fnn {
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| 0 <= i < nbits && new_inner.bitmap@.set_bits.contains(i) implies
            0 < #[trigger] new_inner.refcount@[i] <= 255 by {
            assert(i != fnn);
            assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
        };
        assert forall|i: int| nbits <= i < new_inner.refcount@.len() implies
            #[trigger] new_inner.refcount@[i] == 0 by {
            assert(i != fnn);
            assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
        };
    };

    // allocated_frames: drop `pa`.
    assert(new_inner@.allocated_frames =~= old_inner@.allocated_frames.remove(pa)) by {
        assert forall|addr: int| new_inner@.allocated_frames.contains(addr) implies
            #[trigger] old_inner@.allocated_frames.remove(pa).contains(addr) by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(old_inner.bitmap@.set_bits.contains(i) && i != fnn);
            assert(addr == i * ps);
            if addr == pa {
                assert(i == fnn) by (nonlinear_arith) requires ps > 0, i * ps == fnn * ps, addr == i * ps, pa == fnn * ps, addr == pa;
            }
        }
        assert forall|addr: int| old_inner@.allocated_frames.remove(pa).contains(addr) implies
            #[trigger] new_inner@.allocated_frames.contains(addr) by {
            let i = choose|i: int|
                #[trigger] old_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            if i == fnn {
                assert(addr == fnn * ps);
            }
            assert(i != fnn);
        }
    };

    // free_frames: add `pa`.
    assert(new_inner@.free_frames =~= old_inner@.free_frames.insert(pa)) by {
        assert forall|addr: int| new_inner@.free_frames.contains(addr) implies
            #[trigger] old_inner@.free_frames.insert(pa).contains(addr) by {
            let i = choose|i: int|
                0 <= i < nbits && !(#[trigger] new_inner.bitmap@.set_bits.contains(i))
                    && addr == frame_addr_of(i);
            assert(addr == i * ps);
            if i == fnn {
                assert(addr == pa);
            } else {
                assert(!old_inner.bitmap@.set_bits.contains(i));
            }
        }
        assert forall|addr: int| old_inner@.free_frames.insert(pa).contains(addr) implies
            #[trigger] new_inner@.free_frames.contains(addr) by {
            if addr == pa {
                assert(0 <= fnn < nbits);
                assert(!new_inner.bitmap@.set_bits.contains(fnn));
            } else {
                let i = choose|i: int|
                    0 <= i < nbits && !(#[trigger] old_inner.bitmap@.set_bits.contains(i))
                        && addr == frame_addr_of(i);
                assert(addr == i * ps);
                assert(i != fnn);
            }
        }
    };

    // refcounts: drop `pa`.
    assert(old_inner@.refcounts.contains_key(pa));
    assert(new_inner@.refcounts =~= old_inner@.refcounts.remove(pa)) by {
        assert forall|addr: int| #[trigger] new_inner@.refcounts.contains_key(addr) implies
            old_inner@.refcounts.remove(pa).contains_key(addr) by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(old_inner.bitmap@.set_bits.contains(i) && i != fnn);
            assert(addr == i * ps);
            if addr == pa {
                assert(i == fnn) by (nonlinear_arith) requires ps > 0, i * ps == fnn * ps, addr == i * ps, pa == fnn * ps, addr == pa;
            }
        }
        assert forall|addr: int| old_inner@.refcounts.remove(pa).contains_key(addr) implies
            #[trigger] new_inner@.refcounts.contains_key(addr) by {
            let i = choose|i: int|
                #[trigger] old_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            if i == fnn {
                assert(addr == pa);
            }
            assert(i != fnn);
        }
        assert forall|addr: int| new_inner@.refcounts.contains_key(addr) implies
            #[trigger] new_inner@.refcounts[addr] == old_inner@.refcounts.remove(pa)[addr] by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(old_inner.bitmap@.set_bits.contains(i) && i != fnn);
            assert(addr == i * ps);
            assert(addr / ps == i) by (nonlinear_arith) requires addr == i * ps, ps > 0;
            assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            if addr == pa {
                assert(i == fnn) by (nonlinear_arith) requires ps > 0, i * ps == fnn * ps, addr == i * ps, pa == fnn * ps, addr == pa;
            }
        }
    };
}

// Booking a previously-free frame at index `fnn` sets its bit and initialises its slot to 1. This
// preserves `internal_inv` and moves `pa` (= `fnn * ps`) from the free set to the allocated set with
// a refcount entry of 1. Used by `book`.
proof fn lemma_refcount_book(old_inner: &Inner, new_inner: &Inner, fnn: int, pa: int)
    requires
        old_inner.inv(),
        0 <= fnn < old_inner.bitmap@.num_bits,
        !old_inner.bitmap@.set_bits.contains(fnn),
        new_inner.refcount@ == old_inner.refcount@.update(fnn, 1),
        new_inner.bitmap.inv(),
        new_inner.bitmap@.num_bits == old_inner.bitmap@.num_bits,
        new_inner.bitmap@.set_bits == old_inner.bitmap@.set_bits.insert(fnn),
        pa == fnn * spec_page_size(),
    ensures
        new_inner.internal_inv(),
        new_inner@.allocated_frames =~= old_inner@.allocated_frames.insert(pa),
        new_inner@.free_frames =~= old_inner@.free_frames.remove(pa),
        new_inner@.refcounts =~= old_inner@.refcounts.insert(pa, 1int),
{
    let ps: int = spec_page_size();
    let nbits: int = old_inner.bitmap@.num_bits;
    assert(ps > 0);
    assert(old_inner.bitmap.inv());
    assert(old_inner.bitmap@.wf());
    assert(new_inner.refcount@.len() == old_inner.refcount@.len());
    assert(new_inner.refcount@[fnn] == 1);
    assert(pa == frame_addr_of(fnn));
    // The previously-free slot was zero.
    assert(old_inner.refcount@[fnn] == 0);
    assert forall|i: int| #[trigger] new_inner.bitmap@.set_bits.contains(i) <==> (
        old_inner.bitmap@.set_bits.contains(i) || i == fnn
    ) by {}

    // internal_inv(new_inner): only slot `fnn` changed; it is now set and equal to 1.
    assert(new_inner.internal_inv()) by {
        assert(new_inner.bitmap.inv());
        assert forall|i: int| 0 <= i < nbits implies (
            #[trigger] new_inner.bitmap@.set_bits.contains(i) <==> new_inner.refcount@[i] > 0
        ) by {
            if i == fnn {
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| 0 <= i < nbits implies (
            !(#[trigger] new_inner.bitmap@.set_bits.contains(i)) <==> new_inner.refcount@[i] == 0
        ) by {
            if i == fnn {
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| 0 <= i < nbits && new_inner.bitmap@.set_bits.contains(i) implies
            0 < #[trigger] new_inner.refcount@[i] <= 255 by {
            if i == fnn {
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| nbits <= i < new_inner.refcount@.len() implies
            #[trigger] new_inner.refcount@[i] == 0 by {
            assert(i != fnn);
            assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
        };
    };

    // allocated_frames: add `pa`.
    assert(new_inner@.allocated_frames =~= old_inner@.allocated_frames.insert(pa)) by {
        assert forall|addr: int| new_inner@.allocated_frames.contains(addr) implies
            #[trigger] old_inner@.allocated_frames.insert(pa).contains(addr) by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            if i == fnn {
                assert(addr == pa);
            } else {
                assert(old_inner.bitmap@.set_bits.contains(i));
            }
        }
        assert forall|addr: int| old_inner@.allocated_frames.insert(pa).contains(addr) implies
            #[trigger] new_inner@.allocated_frames.contains(addr) by {
            if addr == pa {
                assert(new_inner.bitmap@.set_bits.contains(fnn));
            } else {
                let i = choose|i: int|
                    #[trigger] old_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                assert(addr == i * ps);
                assert(new_inner.bitmap@.set_bits.contains(i));
            }
        }
    };

    // free_frames: drop `pa`.
    assert(new_inner@.free_frames =~= old_inner@.free_frames.remove(pa)) by {
        assert forall|addr: int| new_inner@.free_frames.contains(addr) implies
            #[trigger] old_inner@.free_frames.remove(pa).contains(addr) by {
            let i = choose|i: int|
                0 <= i < nbits && !(#[trigger] new_inner.bitmap@.set_bits.contains(i))
                    && addr == frame_addr_of(i);
            assert(addr == i * ps);
            assert(i != fnn);
            assert(!old_inner.bitmap@.set_bits.contains(i));
            if addr == pa {
                assert(i == fnn) by (nonlinear_arith) requires ps > 0, i * ps == fnn * ps, addr == i * ps, pa == fnn * ps, addr == pa;
            }
        }
        assert forall|addr: int| old_inner@.free_frames.remove(pa).contains(addr) implies
            #[trigger] new_inner@.free_frames.contains(addr) by {
            let i = choose|i: int|
                0 <= i < nbits && !(#[trigger] old_inner.bitmap@.set_bits.contains(i))
                    && addr == frame_addr_of(i);
            assert(addr == i * ps);
            if i == fnn {
                assert(addr == pa);
            }
            assert(i != fnn);
            assert(!new_inner.bitmap@.set_bits.contains(i));
        }
    };

    // refcounts: add `pa` with value 1.
    assert(new_inner@.refcounts =~= old_inner@.refcounts.insert(pa, 1int)) by {
        assert forall|addr: int| #[trigger] new_inner@.refcounts.contains_key(addr) implies
            old_inner@.refcounts.insert(pa, 1int).contains_key(addr) by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            if i != fnn {
                assert(old_inner.bitmap@.set_bits.contains(i));
            }
        }
        assert forall|addr: int| old_inner@.refcounts.insert(pa, 1int).contains_key(addr) implies
            #[trigger] new_inner@.refcounts.contains_key(addr) by {
            if addr == pa {
                assert(new_inner.bitmap@.set_bits.contains(fnn));
            } else {
                let i = choose|i: int|
                    #[trigger] old_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                assert(addr == i * ps);
                assert(new_inner.bitmap@.set_bits.contains(i));
            }
        }
        assert forall|addr: int| new_inner@.refcounts.contains_key(addr) implies
            #[trigger] new_inner@.refcounts[addr] == old_inner@.refcounts.insert(pa, 1int)[addr] by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            assert(addr / ps == i) by (nonlinear_arith) requires addr == i * ps, ps > 0;
            if addr == pa {
                assert(i == fnn) by (nonlinear_arith) requires ps > 0, i * ps == fnn * ps, addr == i * ps, pa == fnn * ps, addr == pa;
                assert(new_inner.refcount@[i] == 1);
            } else {
                assert(i != fnn);
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        }
    };
}

// The allocator view is a deterministic function of the bitmap's abstract bits and the refcount
// slice. Two inner states that agree on those agree on their abstract views. Used by transitions
// whose failure path leaves the bitmap view and the refcount slice unchanged (e.g. `alloc`'s
// out-of-memory branch).
proof fn lemma_view_determined(a: &Inner, b: &Inner)
    requires
        a.bitmap@.set_bits == b.bitmap@.set_bits,
        a.bitmap@.num_bits == b.bitmap@.num_bits,
        a.refcount@ == b.refcount@,
    ensures
        a@ == b@,
{
    let ps: int = spec_page_size();
    assert(a@.allocated_frames =~= b@.allocated_frames);
    assert(a@.free_frames =~= b@.free_frames);
    assert(a@.refcounts =~= b@.refcounts) by {
        assert forall|addr: int| #[trigger] a@.refcounts.contains_key(addr) implies
            b@.refcounts.contains_key(addr) by {}
        assert forall|addr: int| #[trigger] b@.refcounts.contains_key(addr) implies
            a@.refcounts.contains_key(addr) by {}
        assert forall|addr: int| a@.refcounts.contains_key(addr) implies
            #[trigger] a@.refcounts[addr] == b@.refcounts[addr] by {}
    };
    assert(a@ == b@);
}

// The set of frame (physical) addresses for the contiguous frame-number range `[lo, hi)`.
// Used by `alloc_contiguous`/`alloc_range` to describe the block of frames booked in one step.
pub open spec fn frame_set(lo: int, hi: int) -> Set<int> {
    Set::new(|addr: int| exists|j: int| lo <= j < hi && addr == #[trigger] frame_addr_of(j))
}

// Booking a previously-free contiguous block `[lo, hi)` sets every bit in the range and
// initialises each slot to 1. This preserves `internal_inv` and moves the block of frame
// addresses from the free set to the allocated set, each with a refcount entry of 1. The range
// generalisation of `lemma_refcount_book`, used by `alloc_contiguous` and `alloc_range`.
proof fn lemma_book_range(old_inner: &Inner, new_inner: &Inner, lo: int, hi: int)
    requires
        old_inner.inv(),
        0 <= lo <= hi <= old_inner.bitmap@.num_bits,
        forall|j: int| lo <= j < hi ==> !old_inner.bitmap@.set_bits.contains(j),
        new_inner.bitmap.inv(),
        new_inner.bitmap@.num_bits == old_inner.bitmap@.num_bits,
        new_inner.bitmap@.set_bits == old_inner.bitmap@.set_bits.union(
            BitmapView::range_set(lo, hi)),
        new_inner.refcount@.len() == old_inner.refcount@.len(),
        forall|j: int| lo <= j < hi ==> new_inner.refcount@[j] == 1,
        forall|i: int| !(lo <= i < hi) ==> new_inner.refcount@[i] == old_inner.refcount@[i],
    ensures
        new_inner.internal_inv(),
        new_inner@.allocated_frames =~= old_inner@.allocated_frames.union(frame_set(lo, hi)),
        new_inner@.free_frames =~= old_inner@.free_frames.difference(frame_set(lo, hi)),
        new_inner@.refcounts =~= old_inner@.refcounts.union_prefer_right(
            Map::new(|addr: int| frame_set(lo, hi).contains(addr), |addr: int| 1int)),
{
    let ps: int = spec_page_size();
    let nbits: int = old_inner.bitmap@.num_bits;
    let fs = frame_set(lo, hi);
    assert(ps > 0);
    assert(old_inner.bitmap.inv());
    assert(old_inner.bitmap@.wf());

    // Membership of `new_inner.bitmap@.set_bits` splits into "was set" or "in the range".
    assert forall|i: int| #[trigger] new_inner.bitmap@.set_bits.contains(i) <==> (
        old_inner.bitmap@.set_bits.contains(i) || (lo <= i < hi)
    ) by {
        assert(BitmapView::range_set(lo, hi).contains(i) == (lo <= i < hi));
    }

    // internal_inv(new_inner): only the range `[lo, hi)` changed; those slots are now set to 1.
    assert(new_inner.internal_inv()) by {
        assert(new_inner.bitmap.inv());
        assert forall|i: int| 0 <= i < nbits implies (
            #[trigger] new_inner.bitmap@.set_bits.contains(i) <==> new_inner.refcount@[i] > 0
        ) by {
            if lo <= i < hi {
                assert(new_inner.refcount@[i] == 1);
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| 0 <= i < nbits implies (
            !(#[trigger] new_inner.bitmap@.set_bits.contains(i)) <==> new_inner.refcount@[i] == 0
        ) by {
            if lo <= i < hi {
                assert(new_inner.refcount@[i] == 1);
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| 0 <= i < nbits && new_inner.bitmap@.set_bits.contains(i) implies
            0 < #[trigger] new_inner.refcount@[i] <= 255 by {
            if lo <= i < hi {
                assert(new_inner.refcount@[i] == 1);
            } else {
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
            }
        };
        assert forall|i: int| nbits <= i < new_inner.refcount@.len() implies
            #[trigger] new_inner.refcount@[i] == 0 by {
            assert(!(lo <= i < hi));
            assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
        };
    };

    // allocated_frames: add the block `fs`.
    assert(new_inner@.allocated_frames =~= old_inner@.allocated_frames.union(fs)) by {
        assert forall|addr: int| new_inner@.allocated_frames.contains(addr) implies
            #[trigger] old_inner@.allocated_frames.union(fs).contains(addr) by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            if old_inner.bitmap@.set_bits.contains(i) {
                assert(old_inner@.allocated_frames.contains(addr));
            } else {
                assert(lo <= i < hi);
                assert(fs.contains(addr));
            }
        }
        assert forall|addr: int| old_inner@.allocated_frames.union(fs).contains(addr) implies
            #[trigger] new_inner@.allocated_frames.contains(addr) by {
            if fs.contains(addr) {
                let j = choose|j: int| lo <= j < hi && addr == frame_addr_of(j);
                assert(new_inner.bitmap@.set_bits.contains(j));
            } else {
                let i = choose|i: int|
                    #[trigger] old_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                assert(addr == i * ps);
                assert(new_inner.bitmap@.set_bits.contains(i));
            }
        }
    };

    // free_frames: drop the block `fs`.
    assert(new_inner@.free_frames =~= old_inner@.free_frames.difference(fs)) by {
        assert forall|addr: int| new_inner@.free_frames.contains(addr) implies
            #[trigger] old_inner@.free_frames.difference(fs).contains(addr) by {
            let i = choose|i: int|
                0 <= i < nbits && !(#[trigger] new_inner.bitmap@.set_bits.contains(i))
                    && addr == frame_addr_of(i);
            assert(addr == i * ps);
            assert(!old_inner.bitmap@.set_bits.contains(i));
            assert(!(lo <= i < hi));
            assert(old_inner@.free_frames.contains(addr));
            if fs.contains(addr) {
                let j = choose|j: int| lo <= j < hi && addr == frame_addr_of(j);
                assert(addr == j * ps);
                assert(i == j) by (nonlinear_arith)
                    requires ps > 0, i * ps == j * ps, addr == i * ps, addr == j * ps;
                assert(false);
            }
        }
        assert forall|addr: int| old_inner@.free_frames.difference(fs).contains(addr) implies
            #[trigger] new_inner@.free_frames.contains(addr) by {
            let i = choose|i: int|
                0 <= i < nbits && !(#[trigger] old_inner.bitmap@.set_bits.contains(i))
                    && addr == frame_addr_of(i);
            assert(addr == i * ps);
            if lo <= i < hi {
                assert(fs.contains(addr));
                assert(false);
            }
            assert(!new_inner.bitmap@.set_bits.contains(i));
        }
    };

    // refcounts: add a `→ 1` entry for every address in `fs`.
    let m = Map::new(|addr: int| fs.contains(addr), |addr: int| 1int);
    assert(new_inner@.refcounts =~= old_inner@.refcounts.union_prefer_right(m)) by {
        assert forall|addr: int| #[trigger] new_inner@.refcounts.contains_key(addr) implies
            old_inner@.refcounts.union_prefer_right(m).contains_key(addr) by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            if old_inner.bitmap@.set_bits.contains(i) {
                assert(old_inner@.refcounts.contains_key(addr));
            } else {
                assert(lo <= i < hi);
                assert(fs.contains(addr));
            }
        }
        assert forall|addr: int|
            old_inner@.refcounts.union_prefer_right(m).contains_key(addr) implies
            #[trigger] new_inner@.refcounts.contains_key(addr) by {
            if fs.contains(addr) {
                let j = choose|j: int| lo <= j < hi && addr == frame_addr_of(j);
                assert(new_inner.bitmap@.set_bits.contains(j));
            } else {
                assert(old_inner@.refcounts.contains_key(addr));
                let i = choose|i: int|
                    #[trigger] old_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                assert(addr == i * ps);
                assert(new_inner.bitmap@.set_bits.contains(i));
            }
        }
        assert forall|addr: int| new_inner@.refcounts.contains_key(addr) implies
            #[trigger] new_inner@.refcounts[addr]
                == old_inner@.refcounts.union_prefer_right(m)[addr] by {
            let i = choose|i: int|
                #[trigger] new_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            assert(addr / ps == i) by (nonlinear_arith) requires addr == i * ps, ps > 0;
            if fs.contains(addr) {
                let j = choose|j: int| lo <= j < hi && addr == frame_addr_of(j);
                assert(addr == j * ps);
                assert(i == j) by (nonlinear_arith)
                    requires ps > 0, i * ps == j * ps, addr == i * ps, addr == j * ps;
                assert(new_inner.refcount@[i] == 1);
                assert(m.contains_key(addr));
            } else {
                assert(!(lo <= i < hi));
                assert(new_inner.refcount@[i] == old_inner.refcount@[i]);
                assert(!m.contains_key(addr));
            }
        }
    };
}

// A full bitmap has no free frames. Used by `alloc`'s out-of-memory branch to discharge the
// `free_frames.is_empty()` postcondition.
proof fn lemma_full_no_free(inner: &Inner)
    requires
        inner.bitmap@.is_full(),
    ensures
        inner@.free_frames =~= Set::<int>::empty(),
{
    assert forall|addr: int| !inner@.free_frames.contains(addr) by {
        if inner@.free_frames.contains(addr) {
            let i = choose|i: int|
                0 <= i < inner.bitmap@.num_bits && !(#[trigger] inner.bitmap@.set_bits.contains(i))
                    && addr == frame_addr_of(i);
            assert(inner.bitmap@.set_bits.contains(i));
            assert(false);
        }
    };
}

}
