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
        // spec_page_size() > 0 is a global fact from old_inner.internal_inv()
        // bitmap.inv() is given
        // forall covered(i): chunks unchanged means is_covered is the same
        assert forall|i: int| self.bitmap@.is_covered(i) implies
            i >= 0 && frame_addr_of(i) <= usize::MAX as int
        by {
            assert(old_inner.bitmap@.is_covered(i));
        }
    }
}

} // verus!
