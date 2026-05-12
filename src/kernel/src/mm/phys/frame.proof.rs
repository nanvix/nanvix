verus! {

use vstd::arithmetic::div_mod::*;

/// Helper: convert a bitmap index to a frame (physical) address.
pub open spec fn frame_addr_of(i: int) -> int {
    i * spec_page_size()
}

impl View for Inner {
    type V = UpoolView;

    closed spec fn view(&self) -> UpoolView {
        UpoolView {
            allocated_frames: Set::new(|addr: int|
                exists|i: int|
                    #[trigger] self.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i)
            ),
            free_frames: Set::new(|addr: int|
                exists|i: int| {
                    &&& #[trigger] self.bitmap@.is_covered(i)
                    &&& !self.bitmap@.set_bits.contains(i)
                    &&& addr == frame_addr_of(i)
                }
            ),
        }
    }
}

impl Inner {
    pub closed spec fn internal_inv(&self) -> bool
    {
        &&& self.bitmap.inv()
        // spec_page_size() is uninterp; record positivity from the
        // FRAME_SIZE assume_specification (result > 0).
        &&& spec_page_size() > 0
        // Every covered bitmap index yields a representable, non-negative frame address.
        &&& forall|i: int| self.bitmap@.is_covered(i) ==> {
            &&& i >= 0
            &&& frame_addr_of(i) <= usize::MAX as int
        }
    }

    /// Lemma: internal_inv() ==> self@.wf().
    proof fn lemma_inv_implies_wf(&self)
        requires self.internal_inv(),
        ensures self@.wf(),
    {
        let ps = spec_page_size();

        // Page-alignment of allocated_frames
        assert forall|addr: int| self@.allocated_frames.contains(addr) implies addr % ps == 0
        by {
            let i = choose|i: int| self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(self.bitmap@.set_bits.contains(i));
            assert(addr == i * ps);
            lemma_mod_multiples_basic(i, ps);
        }
        // Page-alignment of free_frames
        assert forall|addr: int| self@.free_frames.contains(addr) implies addr % ps == 0
        by {
            let i = choose|i: int| self.bitmap@.is_covered(i) && !self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * ps);
            lemma_mod_multiples_basic(i, ps);
        }
        // Disjointness
        assert(self@.allocated_frames.disjoint(self@.free_frames)) by {
            assert forall|addr: int| !(self@.allocated_frames.contains(addr) && self@.free_frames.contains(addr))
            by {
                if self@.allocated_frames.contains(addr) && self@.free_frames.contains(addr) {
                    let i_alloc = choose|i: int| self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                    let i_free = choose|i: int| self.bitmap@.is_covered(i) && !self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                    assert(i_alloc * ps == i_free * ps);
                    vstd::arithmetic::mul::lemma_mul_is_commutative(i_alloc, ps);
                    vstd::arithmetic::mul::lemma_mul_is_commutative(i_free, ps);
                    vstd::arithmetic::mul::lemma_mul_equality_converse(ps, i_alloc, i_free);
                }
            }
        }
        // Non-negative allocated addresses
        assert forall|addr: int| self@.allocated_frames.contains(addr) implies addr >= 0
        by {
            let i = choose|i: int| self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(self.bitmap@.set_bits.contains(i));
            assert(self.bitmap@.is_covered(i));
            assert(i >= 0);
            assert(addr == i * ps);
            vstd::arithmetic::mul::lemma_mul_nonnegative(i, ps);
        }
        // Non-negative free addresses
        assert forall|addr: int| self@.free_frames.contains(addr) implies addr >= 0
        by {
            let i = choose|i: int| self.bitmap@.is_covered(i) && !self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(i >= 0);
            assert(addr == i * ps);
            vstd::arithmetic::mul::lemma_mul_nonnegative(i, ps);
        }
    }

    /// Lemma: when bitmap state is preserved (chunks unchanged, inv() maintained),
    /// internal_inv is preserved from the old state.
    proof fn lemma_internal_inv_preserved(&self, old_inner: &Inner)
        requires
            old_inner.internal_inv(),
            self.bitmap.inv(),
            self.bitmap@.chunks =~= old_inner.bitmap@.chunks,
        ensures
            self.internal_inv(),
    {
        assert forall|i: int| self.bitmap@.is_covered(i) implies
            i >= 0 && frame_addr_of(i) <= usize::MAX as int
        by {
            assert(old_inner.bitmap@.is_covered(i));
        }
    }

    // -----------------------------------------------------------------------
    // Extracted proof lemmas (polish pass)
    // -----------------------------------------------------------------------

    /// When bitmap set_bits and chunks are unchanged, the abstract view is identical.
    proof fn lemma_view_unchanged(&self, old_inner: &Inner)
        requires
            self.bitmap@.set_bits =~= old_inner.bitmap@.set_bits,
            self.bitmap@.chunks =~= old_inner.bitmap@.chunks,
        ensures
            self@.allocated_frames =~= old_inner@.allocated_frames,
            self@.free_frames =~= old_inner@.free_frames,
    {
        assert forall|addr: int|
            self@.allocated_frames.contains(addr)
            <==> old_inner@.allocated_frames.contains(addr)
        by {
            if self@.allocated_frames.contains(addr) {
                let i = choose|i: int|
                    #[trigger] self.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                assert(old_inner.bitmap@.set_bits.contains(i));
            }
            if old_inner@.allocated_frames.contains(addr) {
                let i = choose|i: int|
                    #[trigger] old_inner.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                assert(self.bitmap@.set_bits.contains(i));
            }
        }
        assert forall|addr: int|
            self@.free_frames.contains(addr)
            <==> old_inner@.free_frames.contains(addr)
        by {
            if self@.free_frames.contains(addr) {
                let i = choose|i: int|
                    #[trigger] self.bitmap@.is_covered(i)
                    && !self.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                assert(old_inner.bitmap@.is_covered(i));
                assert(!old_inner.bitmap@.set_bits.contains(i));
            }
            if old_inner@.free_frames.contains(addr) {
                let i = choose|i: int|
                    #[trigger] old_inner.bitmap@.is_covered(i)
                    && !old_inner.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                assert(self.bitmap@.is_covered(i));
                assert(!self.bitmap@.set_bits.contains(i));
            }
        }
    }

    /// frame_addr_of is injective: equal addresses imply equal indices.
    proof fn lemma_frame_addr_injective(i: int, j: int)
        requires
            spec_page_size() > 0,
            frame_addr_of(i) == frame_addr_of(j),
        ensures
            i == j,
    {
        let ps = spec_page_size();
        vstd::arithmetic::mul::lemma_mul_is_commutative(i, ps);
        vstd::arithmetic::mul::lemma_mul_is_commutative(j, ps);
        vstd::arithmetic::mul::lemma_mul_equality_converse(ps, i, j);
    }

    /// After inserting bit `idx` into set_bits (chunks unchanged),
    /// allocated gains fa and free loses fa.
    proof fn lemma_set_bit_updates_view(&self, old_inner: &Inner, idx: int, fa: int)
        requires
            old_inner.internal_inv(),
            self.bitmap.inv(),
            self.bitmap@.chunks =~= old_inner.bitmap@.chunks,
            self.bitmap@.set_bits =~= old_inner.bitmap@.set_bits.insert(idx),
            old_inner.bitmap@.is_covered(idx),
            !old_inner.bitmap@.set_bits.contains(idx),
            fa == frame_addr_of(idx),
        ensures
            old_inner@.free_frames.contains(fa),
            self@.allocated_frames =~= old_inner@.allocated_frames.insert(fa),
            self@.free_frames =~= old_inner@.free_frames.remove(fa),
    {
        let ps = spec_page_size();

        // old free_frames contains fa
        assert(old_inner@.free_frames.contains(fa));

        // allocated = old.insert(fa)
        assert forall|addr: int| self@.allocated_frames.contains(addr) implies
            old_inner@.allocated_frames.contains(addr) || addr == fa
        by {
            let i = choose|i: int|
                #[trigger] self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            if i == idx {
                assert(addr == fa);
            } else {
                assert(old_inner.bitmap@.set_bits.contains(i));
            }
        }
        assert forall|addr: int|
            old_inner@.allocated_frames.contains(addr) || addr == fa
            implies self@.allocated_frames.contains(addr)
        by {
            if addr == fa {
                assert(self.bitmap@.set_bits.contains(idx));
            } else {
                let i = choose|i: int|
                    #[trigger] old_inner.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                assert(self.bitmap@.set_bits.contains(i));
            }
        }
        assert(self@.allocated_frames =~= old_inner@.allocated_frames.insert(fa));

        // free = old.remove(fa)
        assert forall|addr: int| self@.free_frames.contains(addr) implies
            old_inner@.free_frames.contains(addr) && addr != fa
        by {
            let i = choose|i: int|
                #[trigger] self.bitmap@.is_covered(i)
                && !self.bitmap@.set_bits.contains(i)
                && addr == frame_addr_of(i);
            if i == idx {
                assert(self.bitmap@.set_bits.contains(idx));
            }
            assert(i != idx);
            assert(!old_inner.bitmap@.set_bits.contains(i));
            assert(old_inner.bitmap@.is_covered(i));
            if addr == fa {
                Self::lemma_frame_addr_injective(i, idx);
            }
        }
        assert forall|addr: int|
            old_inner@.free_frames.contains(addr) && addr != fa
            implies self@.free_frames.contains(addr)
        by {
            let i = choose|i: int|
                #[trigger] old_inner.bitmap@.is_covered(i)
                && !old_inner.bitmap@.set_bits.contains(i)
                && addr == frame_addr_of(i);
            if i == idx { assert(addr == fa); }
            assert(i != idx);
            assert(!self.bitmap@.set_bits.contains(i));
            assert(self.bitmap@.is_covered(i));
        }
        assert(self@.free_frames =~= old_inner@.free_frames.remove(fa));
    }

    /// After removing bit `idx` from set_bits (chunks unchanged),
    /// allocated loses fa and free gains fa.
    proof fn lemma_clear_bit_updates_view(&self, old_inner: &Inner, idx: int, fa: int)
        requires
            old_inner.internal_inv(),
            self.bitmap.inv(),
            self.bitmap@.chunks =~= old_inner.bitmap@.chunks,
            self.bitmap@.set_bits =~= old_inner.bitmap@.set_bits.remove(idx),
            old_inner.bitmap@.set_bits.contains(idx),
            fa == frame_addr_of(idx),
        ensures
            old_inner@.allocated_frames.contains(fa),
            self@.allocated_frames =~= old_inner@.allocated_frames.remove(fa),
            self@.free_frames =~= old_inner@.free_frames.insert(fa),
    {
        let ps = spec_page_size();

        // old allocated contains fa
        assert(old_inner@.allocated_frames.contains(fa));

        // allocated = old.remove(fa)
        assert forall|addr: int| self@.allocated_frames.contains(addr) implies
            old_inner@.allocated_frames.contains(addr) && addr != fa
        by {
            let i = choose|i: int|
                #[trigger] self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(old_inner.bitmap@.set_bits.contains(i));
            if addr == fa {
                Self::lemma_frame_addr_injective(i, idx);
            }
        }
        assert forall|addr: int|
            old_inner@.allocated_frames.contains(addr) && addr != fa
            implies self@.allocated_frames.contains(addr)
        by {
            let i = choose|i: int|
                #[trigger] old_inner.bitmap@.set_bits.contains(i)
                && addr == frame_addr_of(i);
            if i == idx { assert(addr == fa); }
            assert(self.bitmap@.set_bits.contains(i));
        }
        assert(self@.allocated_frames =~= old_inner@.allocated_frames.remove(fa));

        // free = old.insert(fa)
        assert forall|addr: int| self@.free_frames.contains(addr) implies
            old_inner@.free_frames.contains(addr) || addr == fa
        by {
            let i = choose|i: int|
                #[trigger] self.bitmap@.is_covered(i)
                && !self.bitmap@.set_bits.contains(i)
                && addr == frame_addr_of(i);
            if i == idx {
                assert(addr == fa);
            } else {
                assert(!old_inner.bitmap@.set_bits.contains(i));
                assert(old_inner.bitmap@.is_covered(i));
            }
        }
        assert forall|addr: int|
            old_inner@.free_frames.contains(addr) || addr == fa
            implies self@.free_frames.contains(addr)
        by {
            if addr == fa {
                assert(old_inner.bitmap@.is_covered(idx));
                assert(self.bitmap@.is_covered(idx));
                assert(!self.bitmap@.set_bits.contains(idx));
            } else {
                let i = choose|i: int|
                    #[trigger] old_inner.bitmap@.is_covered(i)
                    && !old_inner.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                if i == idx { assert(addr == fa); }
                assert(!self.bitmap@.set_bits.contains(i));
                assert(self.bitmap@.is_covered(i));
            }
        }
        assert(self@.free_frames =~= old_inner@.free_frames.insert(fa));
    }

    /// When all covered bitmap bits are set, free_frames is empty.
    proof fn lemma_bitmap_full_means_free_empty(&self)
        requires
            forall|i: int| self.bitmap@.is_covered(i) ==> self.bitmap@.is_bit_set(i),
        ensures
            self@.free_frames.is_empty(),
    {
        assert forall|addr: int| !self@.free_frames.contains(addr) by {
            if self@.free_frames.contains(addr) {
                let i = choose|i: int|
                    #[trigger] self.bitmap@.is_covered(i)
                    && !self.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                assert(self.bitmap@.is_bit_set(i));
            }
        }
    }

    /// If idx is not in set_bits, fa = frame_addr_of(idx) is not in allocated_frames.
    proof fn lemma_addr_not_allocated(&self, idx: int, fa: int)
        requires
            fa == frame_addr_of(idx),
            spec_page_size() > 0,
            !self.bitmap@.set_bits.contains(idx),
        ensures
            !self@.allocated_frames.contains(fa),
    {
        assert forall|i: int|
            self.bitmap@.set_bits.contains(i) && fa == frame_addr_of(i)
            implies false
        by {
            if i != idx {
                Self::lemma_frame_addr_injective(i, idx);
            }
        }
    }

    /// If idx is not covered or is already set, fa = frame_addr_of(idx) is not in free_frames.
    proof fn lemma_addr_not_free(&self, idx: int, fa: int)
        requires
            fa == frame_addr_of(idx),
            spec_page_size() > 0,
            !self.bitmap@.is_covered(idx) || self.bitmap@.set_bits.contains(idx),
        ensures
            !self@.free_frames.contains(fa),
    {
        assert forall|i: int|
            self.bitmap@.is_covered(i)
            && !self.bitmap@.set_bits.contains(i)
            && fa == frame_addr_of(i)
            implies false
        by {
            if i != idx {
                Self::lemma_frame_addr_injective(i, idx);
            }
        }
    }

    /// After union-ing bitmap set_bits with range [sfn, efn),
    /// the abstract view reflects spec_alloc_range.
    proof fn lemma_alloc_range_updates_view(&self, old_inner: &Inner, sfn: int, efn: int)
        requires
            old_inner.internal_inv(),
            self.bitmap.inv(),
            self.bitmap@.chunks =~= old_inner.bitmap@.chunks,
            self.bitmap@.set_bits =~= old_inner.bitmap@.set_bits.union(
                vstd::set_lib::set_int_range(sfn, efn)
            ),
            forall|j: int| sfn <= j < efn ==> old_inner.bitmap@.is_covered(j),
            forall|j: int| sfn <= j < efn ==> !old_inner.bitmap@.set_bits.contains(j),
        ensures
            ({
                let ps = spec_page_size();
                let frames = vstd::set_lib::set_int_range(sfn, efn).map(|i: int| i * ps);
                &&& frames.subset_of(old_inner@.free_frames)
                &&& self@.allocated_frames =~= old_inner@.allocated_frames.union(frames)
                &&& self@.free_frames =~= old_inner@.free_frames.difference(frames)
            }),
    {
        let ps = spec_page_size();
        let range = vstd::set_lib::set_int_range(sfn, efn);
        let frames = range.map(|i: int| i * ps);

        // 1. frames ⊆ old.free_frames
        assert forall|addr: int| frames.contains(addr)
            implies old_inner@.free_frames.contains(addr)
        by {
            let j = choose|j: int| range.contains(j) && addr == j * ps;
            assert(sfn <= j && j < efn);
            assert(old_inner.bitmap@.is_covered(j));
            assert(!old_inner.bitmap@.set_bits.contains(j));
            assert(addr == frame_addr_of(j));
        }

        // 2. allocated = old.allocated ∪ frames
        assert forall|addr: int|
            self@.allocated_frames.contains(addr) <==>
            old_inner@.allocated_frames.union(frames).contains(addr)
        by {
            if self@.allocated_frames.contains(addr) {
                let i = choose|i: int|
                    self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                if old_inner.bitmap@.set_bits.contains(i) {
                    assert(old_inner@.allocated_frames.contains(addr));
                } else {
                    assert(range.contains(i));
                    assert(frames.contains(addr));
                }
            }
            if old_inner@.allocated_frames.contains(addr) {
                let i = choose|i: int|
                    old_inner.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                assert(self.bitmap@.set_bits.contains(i));
            }
            if frames.contains(addr) {
                let j = choose|j: int| range.contains(j) && addr == j * ps;
                assert(self.bitmap@.set_bits.contains(j));
                assert(addr == frame_addr_of(j));
            }
        }

        // 3. free = old.free \ frames
        assert forall|addr: int|
            self@.free_frames.contains(addr) <==>
            old_inner@.free_frames.difference(frames).contains(addr)
        by {
            if self@.free_frames.contains(addr) {
                let i = choose|i: int|
                    self.bitmap@.is_covered(i)
                    && !self.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                assert(old_inner.bitmap@.is_covered(i));
                assert(!old_inner.bitmap@.set_bits.contains(i));
                assert(old_inner@.free_frames.contains(addr));
                assert(!range.contains(i));
                if frames.contains(addr) {
                    let j = choose|j: int| range.contains(j) && addr == j * ps;
                    Self::lemma_frame_addr_injective(i, j);
                    assert(false);
                }
            }
            if old_inner@.free_frames.contains(addr) && !frames.contains(addr) {
                let i = choose|i: int|
                    old_inner.bitmap@.is_covered(i)
                    && !old_inner.bitmap@.set_bits.contains(i)
                    && addr == frame_addr_of(i);
                assert(self.bitmap@.is_covered(i));
                if range.contains(i) {
                    assert(frames.contains(addr));
                    assert(false);
                }
                assert(!self.bitmap@.set_bits.contains(i));
            }
        }

        assert(self@.allocated_frames =~= old_inner@.allocated_frames.union(frames));
        assert(self@.free_frames =~= old_inner@.free_frames.difference(frames));
    }

    /// Prove that frame quotients sum is bounded when region start + size ≤ usize::MAX.
    proof fn lemma_frame_quotients_bounded(start: int, size: int, ps: int)
        requires
            ps >= 2,
            start >= 0,
            size >= 0,
            start % ps == 0,
            size % ps == 0,
            start + size <= usize::MAX as int,
        ensures
            start / ps + size / ps <= usize::MAX as int,
    {
        lemma_fundamental_div_mod(start, ps);
        lemma_fundamental_div_mod(size, ps);
        vstd::arithmetic::mul::lemma_mul_inequality(1, ps, start / ps);
        vstd::arithmetic::mul::lemma_mul_is_commutative(ps, start / ps);
        vstd::arithmetic::mul::lemma_mul_inequality(1, ps, size / ps);
        vstd::arithmetic::mul::lemma_mul_is_commutative(ps, size / ps);
    }

    /// Prove end_frame_number properties for alloc_range:
    /// efn == (start + size) / ps and efn * ps == start + size.
    proof fn lemma_end_frame_number_properties(start: int, size: int, sfn: int, nf: int, ps: int)
        requires
            ps >= 2,
            start >= 0,
            size >= 0,
            start % ps == 0,
            size % ps == 0,
            start + size <= usize::MAX as int,
            sfn == start / ps,
            nf == size / ps,
        ensures
            sfn + nf == (start + size) / ps,
            (sfn + nf) * ps == start + size,
            (sfn + nf) * ps <= usize::MAX as int,
    {
        lemma_fundamental_div_mod(start, ps);
        lemma_fundamental_div_mod(size, ps);
        vstd::arithmetic::mul::lemma_mul_is_commutative(ps, nf);
        assert(nf * ps == size);
        lemma_hoist_over_denominator(start, nf, ps as nat);
        assert(sfn + nf == (start + size) / ps);
        vstd::arithmetic::mul::lemma_mul_is_commutative(ps, sfn);
        assert(sfn * ps == start);
        assert(start + size == sfn * ps + nf * ps);
        assert(start + size == ((sfn) + (nf)) * (ps as int)) by (nonlinear_arith)
            requires
                start == (sfn) * ps,
                size == (nf) * ps,
        {}
        assert(((sfn) + (nf)) * (ps as int) == (sfn + nf) * (ps as int)) by (nonlinear_arith)
            requires true,
        {}
    }

    /// Prove that bitmap coverage transfers when chunks are unchanged.
    proof fn lemma_coverage_transfers(&self, old_inner: &Inner, sfn: int, efn: int)
        requires
            self.bitmap@.chunks =~= old_inner.bitmap@.chunks,
            forall|j: int| sfn <= j < efn ==> old_inner.bitmap@.is_covered(j),
        ensures
            forall|j: int| sfn <= j < efn ==> self.bitmap@.is_covered(j),
    {
        assert forall|j: int| sfn <= j < efn implies self.bitmap@.is_covered(j) by {
            assert(old_inner.bitmap@.is_covered(j));
        }
    }

    /// Prove set_int_range grows by one element.
    proof fn lemma_range_insert_step(sfn: int, idx: int)
        ensures
            vstd::set_lib::set_int_range(sfn, (idx + 1) as int) =~=
                vstd::set_lib::set_int_range(sfn, idx as int).insert(idx as int),
    {
        assert forall|x: int|
            vstd::set_lib::set_int_range(sfn, (idx + 1) as int).contains(x) <==>
            vstd::set_lib::set_int_range(sfn, idx as int).insert(idx as int).contains(x)
        by {}
    }
}

} // verus!
