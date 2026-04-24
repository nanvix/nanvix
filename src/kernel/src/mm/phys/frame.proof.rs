verus! {

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
        // Every covered bitmap index yields a representable, non-negative frame address.
        &&& forall|i: int| self.bitmap@.is_covered(i) ==> {
            &&& i >= 0
            &&& frame_addr_of(i) <= usize::MAX as int
        }
    }

    /// Lemma: bitmap.inv() + internal_inv() ==> self@.wf().
    proof fn lemma_inv_implies_wf(&self)
        requires self.internal_inv(),
        ensures self@.wf(),
    {
        // Page-alignment of allocated_frames
        assert forall|addr: int| self@.allocated_frames.contains(addr) implies addr % spec_page_size() == 0
        by {
            let i = choose|i: int| self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(self.bitmap@.set_bits.contains(i));
            assert(addr == i * spec_page_size());
        }
        // Page-alignment of free_frames
        assert forall|addr: int| self@.free_frames.contains(addr) implies addr % spec_page_size() == 0
        by {
            let i = choose|i: int| self.bitmap@.is_covered(i) && !self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(addr == i * spec_page_size());
        }
        // Disjointness
        assert(self@.allocated_frames.disjoint(self@.free_frames)) by {
            assert forall|addr: int| !(self@.allocated_frames.contains(addr) && self@.free_frames.contains(addr))
            by {
                if self@.allocated_frames.contains(addr) && self@.free_frames.contains(addr) {
                    let i_alloc = choose|i: int| self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                    let i_free = choose|i: int| self.bitmap@.is_covered(i) && !self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
                    assert(i_alloc * spec_page_size() == i_free * spec_page_size());
                    // Since page_size > 0, i_alloc == i_free
                    assert(i_alloc == i_free);
                    // Contradiction: i_alloc in set_bits but i_free not in set_bits
                }
            }
        }
        // Non-negative allocated addresses
        assert forall|addr: int| self@.allocated_frames.contains(addr) implies addr >= 0
        by {
            let i = choose|i: int| self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            // set_bits ⊆ covered (from bitmap wf), so i is covered, so i >= 0
            assert(self.bitmap@.set_bits.contains(i));
            assert(self.bitmap@.is_covered(i));
            assert(i >= 0);
        }
        // Non-negative free addresses
        assert forall|addr: int| self@.free_frames.contains(addr) implies addr >= 0
        by {
            let i = choose|i: int| self.bitmap@.is_covered(i) && !self.bitmap@.set_bits.contains(i) && addr == frame_addr_of(i);
            assert(i >= 0);
        }
    }
}

} // verus!
