// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Bitmap - Specifications
//
// This file contains specification functions, BitmapView, and View trait for Bitmap.

verus! {

//==================================================================================================
// BitmapView - Abstract Specification Model
//==================================================================================================

/// A view of the Bitmap as a set of indices where bits are set.
#[verifier::ext_equal]
pub struct BitmapView {
    /// Number of bits in the bitmap.
    pub num_bits: int,
    /// Set of indices where bits are set (0-indexed).
    /// Invariant: all elements are in [0, num_bits).
    pub set_bits: Set<int>,
}

impl BitmapView {
    /// Returns the usage (count of set bits) in the bitmap view.
    /// Requires set_bits to be finite (enforced by Bitmap::inv()).
    pub open spec fn usage(&self) -> int {
        self.set_bits.len() as int
    }

    /// Returns true if there exists at least one unset bit.
    pub open spec fn has_free_bit(&self) -> bool {
        exists|i: int| 0 <= i < self.num_bits && !self.set_bits.contains(i)
    }

    /// Returns true if the bitmap is full (all bits set).
    pub open spec fn is_full(&self) -> bool {
        forall|i: int| 0 <= i < self.num_bits ==> self.set_bits.contains(i)
    }

    /// Returns true if the bitmap is empty (no bits set).
    pub open spec fn is_empty(&self) -> bool {
        self.set_bits == Set::<int>::empty()
    }

    /// Returns true if a specific bit is set.
    pub open spec fn is_bit_set(&self, index: int) -> bool {
        self.set_bits.contains(index)
    }

    /// Helper: Create a set of indices in range [start, end).
    pub open spec fn range_set(start: int, end: int) -> Set<int> {
        Set::new(|i: int| start <= i < end)
    }

    /// Well-formedness: set_bits only contains valid indices.
    pub open spec fn wf(&self) -> bool {
        forall|i: int| self.set_bits.contains(i) ==> 0 <= i < self.num_bits
    }

    /// Helper spec function: check if all bits in range [start, end) are set.
    pub open spec fn all_bits_set_in_range(&self, start: int, end: int) -> bool {
        forall|i: int| start <= i < end ==> self.is_bit_set(i)
    }

    /// Helper spec function: check if all bits in range [start, end) are not set.
    pub open spec fn all_bits_unset_in_range(&self, start: int, end: int) -> bool {
        forall|i: int| start <= i < end ==> !self.is_bit_set(i)
    }

    /// Helper spec function: check if there exists a contiguous range of n free bits starting at start.
    pub open spec fn has_free_range_at(&self, start: int, n: int) -> bool {
        &&& 0 <= start
        &&& start + n <= self.num_bits
        &&& self.all_bits_unset_in_range(start, start + n)
    }

    /// Helper spec function: check if there exists a contiguous range of n free bits.
    pub open spec fn exists_contiguous_free_range(&self, n: int) -> bool {
        exists|start: int|
            #![trigger self.has_free_range_at(start, n)]
            self.has_free_range_at(start, n)
    }

    /// A subset of a finite set is finite.
    pub proof fn lemma_set_bits_finite(&self)
        requires
            self.wf(),
            self.num_bits >= 0,
        ensures
            self.set_bits.finite(),
    {
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, self.num_bits);
        vstd::set_lib::lemma_int_range(0, self.num_bits);

        assert(self.set_bits.subset_of(full_range)) by {
            assert forall|i: int| #![auto] self.set_bits.contains(i) implies full_range.contains(
                i,
            ) by {}
        }

        vstd::set_lib::lemma_set_subset_finite(full_range, self.set_bits);
    }

    //==================================================================================================
    // Lemmas: Finiteness
    //==================================================================================================

    /// range_set(lo, hi) is finite when lo <= hi.
    pub proof fn lemma_range_set_finite(lo: int, hi: int)
        requires
            lo <= hi,
        ensures
            BitmapView::range_set(lo, hi).finite(),
    {
        vstd::set_lib::lemma_int_range(lo, hi);
        assert(BitmapView::range_set(lo, hi) =~= vstd::set_lib::set_int_range(lo, hi)) by {
            assert forall|i: int|
                BitmapView::range_set(lo, hi).contains(i) == vstd::set_lib::set_int_range(
                    lo,
                    hi,
                ).contains(i) by {}
        }
    }

    //==================================================================================================
    // Lemmas: Cardinality
    //==================================================================================================

    /// range_set cardinality equals range size.
    pub proof fn lemma_range_set_len(lo: int, hi: int)
        requires
            lo <= hi,
        ensures
            BitmapView::range_set(lo, hi).len() == hi - lo,
    {
        vstd::set_lib::lemma_int_range(lo, hi);
        assert(BitmapView::range_set(lo, hi) =~= vstd::set_lib::set_int_range(lo, hi)) by {
            assert forall|i: int|
                BitmapView::range_set(lo, hi).contains(i) == vstd::set_lib::set_int_range(
                    lo,
                    hi,
                ).contains(i) by {}
        }
    }

    //==================================================================================================
    // Lemmas: Free Range Properties
    //==================================================================================================

    /// If a free range of size n exists starting at p, then usage <= number_of_bits - n.
    proof fn lemma_free_range_implies_usage_bound(&self, p: int, n: int)
        requires
            self.wf(),
            self.has_free_range_at(p, n),
            n > 0,
        ensures
            self.usage() <= self.num_bits - n,
    {
        let num_bits: int = self.num_bits;
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, num_bits);
        vstd::set_lib::lemma_int_range(0, num_bits);
        let free_range: Set<int> = vstd::set_lib::set_int_range(p, p + n);
        vstd::set_lib::lemma_int_range(p, p + n);
        let available_range: Set<int> = full_range.difference(free_range);

        assert(self.set_bits.subset_of(available_range)) by {
            assert forall|i: int|
                #![auto]
                self.set_bits.contains(i) implies available_range.contains(i) by {
                if p <= i && i < p + n {
                    assert(!self.is_bit_set(i));
                }
            }
        }

        vstd::set_lib::lemma_set_subset_finite(full_range, available_range);
        vstd::set_lib::lemma_len_subset(self.set_bits, available_range);

        assert(free_range.subset_of(full_range)) by {
            assert forall|i: int| free_range.contains(i) implies full_range.contains(i) by {}
        }
        vstd::set_lib::lemma_len_difference(full_range, free_range);
        assert(full_range.intersect(free_range) =~= free_range);
        assert(full_range =~= available_range.union(free_range));
        assert(available_range.intersect(free_range) =~= Set::empty());
        vstd::set_lib::lemma_set_disjoint_lens(available_range, free_range);
    }

    //==================================================================================================
    // Lemmas: Miscellaneous
    //==================================================================================================

    /// Lemma: If set_bits ⊆ [0, n) and there's an element x in [0, n) not in set_bits,
    /// then |set_bits| < n, so inserting one element still satisfies |set_bits| <= n.
    pub proof fn lemma_insert_preserves_usage_bound(&self, x: int)
        requires
            self.wf(),
            0 <= x < self.num_bits,
            !self.set_bits.contains(x),
        ensures
            self.set_bits.insert(x).len() <= self.num_bits,
    {
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, self.num_bits);
        vstd::set_lib::lemma_int_range(0, self.num_bits);
        assert(self.set_bits.subset_of(full_range)) by {
            assert forall|i: int| #![auto] self.set_bits.contains(i) implies full_range.contains(
                i,
            ) by {}
        }
        self.set_bits.lemma_subset_not_in_lt(full_range, x);
    }

    /// If usage equals number_of_bits, all bits are set.
    pub proof fn lemma_usage_equals_number_of_bits_implies_full(&self)
        requires
            self.wf(),
            self.usage() == self.num_bits,
        ensures
            forall|i: int| 0 <= i < self.num_bits ==> self.is_bit_set(i),
    {
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, self.num_bits);
        vstd::set_lib::lemma_int_range(0, self.num_bits);
        assert(self.set_bits.subset_of(full_range)) by {
            assert forall|i: int| #![auto] self.set_bits.contains(i) implies full_range.contains(
                i,
            ) by {}
        }
        assert forall|i: int| 0 <= i < self.num_bits implies self.set_bits.contains(i) by {
            if !self.set_bits.contains(i) {
                let reduced: Set<int> = full_range.remove(i);
                assert(self.set_bits.subset_of(reduced)) by {
                    assert forall|j: int|
                        #![auto]
                        self.set_bits.contains(j) implies reduced.contains(j) by {}
                }
                vstd::set_lib::lemma_len_subset(self.set_bits, reduced);
                vstd::set_lib::lemma_set_subset_finite(full_range, reduced);
            }
        }
    }

    /// If usage() < number_of_bits(), then the bitmap is not full.
    pub proof fn lemma_usage_less_than_capacity_means_not_full(&self)
        requires
            self.wf(),
            self.usage() < self.num_bits,
        ensures
            !self.is_full(),
    {
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, self.num_bits);
        vstd::set_lib::lemma_int_range(0, self.num_bits);
        assert(self.set_bits.subset_of(full_range)) by {
            assert forall|i: int| #![auto] self.set_bits.contains(i) implies full_range.contains(
                i,
            ) by {}
        }
        vstd::set_lib::lemma_len_subset(self.set_bits, full_range);
        let diff: Set<int> = full_range.difference(self.set_bits);
        vstd::set_lib::lemma_len_difference(full_range, self.set_bits);
        assert(full_range.intersect(self.set_bits) =~= self.set_bits);
        assert(full_range =~= diff.union(self.set_bits));
        vstd::set_lib::lemma_set_disjoint_lens(diff, self.set_bits);
        vstd::set_lib::lemma_set_empty_equivalency_len(diff);
        let i: int = diff.choose();
        assert(!self.set_bits.contains(i));
    }

    /// If a specific bit is unset, then has_free_bit() is true.
    pub proof fn lemma_unset_bit_implies_has_free_bit(&self, i: int)
        requires
            self.wf(),
            0 <= i < self.num_bits,
            !self.is_bit_set(i),
        ensures
            self.has_free_bit(),
    {
    }

    /// Lemma: if all bits are set, bitmap is full.
    pub proof fn lemma_all_bits_set_means_full(&self)
        requires
            self.wf(),
            forall|i: int| 0 <= i < self.num_bits ==> self.is_bit_set(i),
        ensures
            self.is_full(),
    {
        assert forall|i: int| 0 <= i < self.num_bits implies self.set_bits.contains(i) by {
            assert(self.is_bit_set(i));
        }
    }

    /// has_free_bit implies exists_contiguous_free_range(1).
    pub proof fn lemma_has_free_bit_implies_exists_free_range_1(&self)
        requires
            self.wf(),
            self.has_free_bit(),
        ensures
            self.exists_contiguous_free_range(1),
    {
        let i = choose|i: int| 0 <= i < self.num_bits && !self.set_bits.contains(i);
        assert(self.has_free_range_at(i, 1));
    }

    /// If set_bits are equal, has_free_range_at returns the same result.
    pub proof fn lemma_set_bits_equal_has_free_range_at_equal(&self, other: &Self, p: int, n: int)
        requires
            self.wf(),
            other.wf(),
            self.set_bits =~= other.set_bits,
            self.num_bits == other.num_bits,
        ensures
            self.has_free_range_at(p, n) == other.has_free_range_at(p, n),
    {
    }

    /// If set_bits are equal, exists_contiguous_free_range returns the same result.
    pub proof fn lemma_set_bits_equal_exists_free_range_equal(&self, other: &Self, n: int)
        requires
            self.wf(),
            other.wf(),
            self.set_bits =~= other.set_bits,
            self.num_bits == other.num_bits,
        ensures
            self.exists_contiguous_free_range(n) == other.exists_contiguous_free_range(n),
    {
        assert forall|p: int|
            #![trigger self.has_free_range_at(p, n)]
            self.has_free_range_at(p, n) == other.has_free_range_at(p, n) by {
            self.lemma_set_bits_equal_has_free_range_at_equal(other, p, n);
        }
        if self.exists_contiguous_free_range(n) {
            let p = choose|p: int| #[trigger] self.has_free_range_at(p, n);
        }
        if other.exists_contiguous_free_range(n) {
            let p = choose|p: int| #[trigger] other.has_free_range_at(p, n);
        }
    }
}

//==================================================================================================
// View Implementation for Bitmap
//==================================================================================================

impl View for Bitmap {
    type V = BitmapView;

    closed spec fn view(&self) -> BitmapView {
        BitmapView {
            num_bits: self.number_of_bits as int,
            set_bits: Set::new(
                |i: int| 0 <= i < self.number_of_bits as int && Self::bit_at(self.bits@, i),
            ),
        }
    }
}

//==================================================================================================
// Bitmap Specification Functions
//==================================================================================================

impl Bitmap {
    /// Helper spec function: get the bit value at a specific index from raw bytes.
    spec fn bit_at(bytes: Seq<u8>, bit_index: int) -> bool {
        let word: int = bit_index / (u8::BITS as int);
        let bit: int = bit_index % (u8::BITS as int);
        if 0 <= bit_index && word < bytes.len() {
            (bytes[word] & (1u8 << bit)) != 0
        } else {
            false
        }
    }

    pub open spec fn inv(&self) -> bool {
        &&& self@.wf()
        &&& self.internal_inv()
    }

    /// Invariant: the bitmap's state is well-formed.
    pub closed spec fn internal_inv(&self) -> bool {
        &&& self.bits.inv()
        &&& self@.num_bits > 0
        &&& self@.num_bits == self.bits@.len() * (u8::BITS as int)
        &&& self@.num_bits < u32::MAX as int
        &&& self@.wf()  // set_bits only contains valid indices
        &&& self@.set_bits.finite()  // set_bits is finite (required for len())
        &&& self@.usage() <= self@.num_bits
        &&& self.number_of_bits as int == self@.num_bits
        &&& self.usage as int == self@.usage()
        &&& self.next_free as int <= self@.num_bits
    }
}

} // verus!
