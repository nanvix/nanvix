// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Bitmap - Proofs
//
// This file contains lemmas and proof functions for Bitmap.
// Uses Set<int> as the primary abstraction.



verus! {

impl Bitmap {
    //==================================================================================================
    // Lemmas: Finiteness
    //==================================================================================================

    /// Lemma: A subset of a finite set is finite.
    /// set_bits ⊆ [0, num_bits) and [0, num_bits) is finite, so set_bits is finite.
    pub proof fn lemma_set_bits_finite(&self)
        requires
            self@.wf(),
            self@.num_bits >= 0,
        ensures
            self@.set_bits.finite(),
    {
        // The set [0, num_bits) is finite.
        // set_bits is a subset of [0, num_bits) by wf().
        // A subset of a finite set is finite.
        
        // Use set_int_range to get a finite set.
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, self@.num_bits);
        
        // Prove full_range is finite using vstd lemma.
        vstd::set_lib::lemma_int_range(0, self@.num_bits);
        
        // Prove set_bits ⊆ full_range.
        assert(self@.set_bits.subset_of(full_range)) by {
            assert forall|i: int| #![auto] self@.set_bits.contains(i) implies full_range.contains(i) by {
                // From wf(): set_bits.contains(i) ==> 0 <= i < num_bits
            }
        }
        
        // A subset of a finite set is finite.
        vstd::set_lib::lemma_set_subset_finite(full_range, self@.set_bits);
    }

    /// Lemma: Extensional equality preserves finiteness.
    /// If s1 =~= s2 and s2 is finite, then s1 is finite.
    pub proof fn lemma_ext_equal_finite(s1: Set<int>, s2: Set<int>)
        requires
            s1 =~= s2,
            s2.finite(),
        ensures
            s1.finite(),
    {
        // s1 =~= s2 means forall|a| s1.contains(a) == s2.contains(a)
        // So s1 ⊆ s2, and by lemma_set_subset_finite, s1.finite()
        assert(s1.subset_of(s2)) by {
            assert forall|a: int| s1.contains(a) implies s2.contains(a) by {}
        }
        vstd::set_lib::lemma_set_subset_finite(s2, s1);
    }

    /// Lemma: Extensional equality preserves length.
    /// If s1 =~= s2 and both are finite, then s1.len() == s2.len().
    pub proof fn lemma_ext_equal_len(s1: Set<int>, s2: Set<int>)
        requires
            s1 =~= s2,
            s2.finite(),
        ensures
            s1.finite(),
            s1.len() == s2.len(),
    {
        Self::lemma_ext_equal_finite(s1, s2);
        // s1 ⊆ s2 and s2 ⊆ s1, so |s1| <= |s2| and |s2| <= |s1|.
        assert(s1.subset_of(s2)) by {
            assert forall|a: int| s1.contains(a) implies s2.contains(a) by {}
        }
        assert(s2.subset_of(s1)) by {
            assert forall|a: int| s2.contains(a) implies s1.contains(a) by {}
        }
        vstd::set_lib::lemma_len_subset(s1, s2);
        vstd::set_lib::lemma_len_subset(s2, s1);
    }

    /// Lemma: range_set(lo, hi) is finite when lo <= hi.
    pub proof fn lemma_range_set_finite(lo: int, hi: int)
        requires
            lo <= hi,
        ensures
            BitmapView::range_set(lo, hi).finite(),
    {
        // range_set(lo, hi) = { i | lo <= i < hi } = set_int_range(lo, hi)
        vstd::set_lib::lemma_int_range(lo, hi);
        // Prove equality.
        assert(BitmapView::range_set(lo, hi) =~= vstd::set_lib::set_int_range(lo, hi)) by {
            assert forall|i: int| BitmapView::range_set(lo, hi).contains(i) ==
                vstd::set_lib::set_int_range(lo, hi).contains(i) by {}
        }
        Self::lemma_ext_equal_finite(BitmapView::range_set(lo, hi), vstd::set_lib::set_int_range(lo, hi));
    }

    /// Lemma: Empty set is finite.
    pub proof fn lemma_empty_set_finite()
        ensures
            Set::<int>::empty().finite(),
    {
        // Axiom from vstd.
    }

    /// Lemma: Inserting into a finite set produces a finite set.
    pub proof fn lemma_insert_finite(s: Set<int>, x: int)
        requires
            s.finite(),
        ensures
            s.insert(x).finite(),
    {
        // Follows from vstd axiom_set_insert_finite.
    }

    /// Lemma: Removing from a finite set produces a finite set.
    pub proof fn lemma_remove_finite(s: Set<int>, x: int)
        requires
            s.finite(),
        ensures
            s.remove(x).finite(),
    {
        // Follows from vstd axiom_set_remove_finite.
    }

    /// Lemma: Union of two finite sets is finite.
    pub proof fn lemma_union_finite(s1: Set<int>, s2: Set<int>)
        requires
            s1.finite(),
            s2.finite(),
        ensures
            s1.union(s2).finite(),
    {
        // Follows from vstd axiom_set_union_finite.
    }

    /// Lemma: Difference of a finite set and any set is finite.
    pub proof fn lemma_difference_finite(s1: Set<int>, s2: Set<int>)
        requires
            s1.finite(),
        ensures
            s1.difference(s2).finite(),
    {
        // Follows from vstd axiom_set_difference_finite.
    }

    //==================================================================================================
    // Lemmas: Cardinality
    //==================================================================================================

    /// Lemma: Inserting a new element increases cardinality by 1.
    pub proof fn lemma_insert_len(s: Set<int>, x: int)
        requires
            s.finite(),
            !s.contains(x),
        ensures
            s.insert(x).len() == s.len() + 1,
    {
        // Follows from vstd axiom_set_insert_len.
    }

    /// Lemma: Inserting an existing element doesn't change cardinality.
    pub proof fn lemma_insert_same_len(s: Set<int>, x: int)
        requires
            s.finite(),
            s.contains(x),
        ensures
            s.insert(x).len() == s.len(),
    {
        assert(s.insert(x) =~= s);
    }

    /// Lemma: Disjoint union has cardinality equal to sum of cardinalities.
    pub proof fn lemma_disjoint_union_len(s1: Set<int>, s2: Set<int>)
        requires
            s1.finite(),
            s2.finite(),
            s1.disjoint(s2),
        ensures
            s1.union(s2).len() == s1.len() + s2.len(),
    {
        vstd::set_lib::lemma_set_disjoint_lens(s1, s2);
    }

    /// Lemma: range_set cardinality equals range size.
    pub proof fn lemma_range_set_len(lo: int, hi: int)
        requires
            lo <= hi,
        ensures
            BitmapView::range_set(lo, hi).len() == hi - lo,
    {
        // range_set(lo, hi) = { i | lo <= i < hi } = set_int_range(lo, hi).
        vstd::set_lib::lemma_int_range(lo, hi);
        // Prove equality.
        assert(BitmapView::range_set(lo, hi) =~= vstd::set_lib::set_int_range(lo, hi)) by {
            assert forall|i: int| BitmapView::range_set(lo, hi).contains(i) ==
                vstd::set_lib::set_int_range(lo, hi).contains(i) by {}
        }
    }

    /// Lemma: Removing an existing element decreases cardinality by 1.
    pub proof fn lemma_remove_len(s: Set<int>, x: int)
        requires
            s.finite(),
            s.contains(x),
        ensures
            s.remove(x).len() == s.len() - 1,
    {
        // Follows from vstd axiom_set_remove_len.
    }

    /// Lemma: Removing a non-existing element doesn't change cardinality.
    pub proof fn lemma_remove_same_len(s: Set<int>, x: int)
        requires
            s.finite(),
            !s.contains(x),
        ensures
            s.remove(x).len() == s.len(),
    {
        assert(s.remove(x) =~= s);
    }

    /// Lemma: Empty set has cardinality 0.
    pub proof fn lemma_empty_len()
        ensures
            Set::<int>::empty().len() == 0,
    {
        // Follows from vstd axiom_set_empty_len.
    }

    /// Lemma: If set_bits ⊆ [0, n) and there's an element x in [0, n) not in set_bits,
    /// then |set_bits| < n, so inserting one element still satisfies |set_bits| <= n.
    pub proof fn lemma_insert_preserves_usage_bound(&self, x: int)
        requires
            self.inv(),
            0 <= x < self@.number_of_bits(),
            !self@.set_bits.contains(x),
        ensures
            self@.set_bits.insert(x).len() <= self@.number_of_bits(),
    {
        // set_bits ⊆ [0, num_bits) by wf().
        // x is in [0, num_bits) but not in set_bits.
        // So set_bits is a strict subset of [0, num_bits).
        
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, self@.num_bits);
        vstd::set_lib::lemma_int_range(0, self@.num_bits);
        
        // set_bits ⊆ full_range.
        assert(self@.set_bits.subset_of(full_range)) by {
            assert forall|i: int| #![auto] self@.set_bits.contains(i) implies full_range.contains(i) by {}
        }
        
        // x is in full_range but not in set_bits.
        assert(full_range.contains(x));
        
        // By lemma_subset_not_in_lt: |set_bits| < |full_range| = num_bits.
        self@.set_bits.lemma_subset_not_in_lt(full_range, x);
        assert(self@.set_bits.len() < full_range.len());
        
        // After inserting x: |set_bits.insert(x)| = |set_bits| + 1 <= num_bits.
        Self::lemma_insert_len(self@.set_bits, x);
    }

    //==================================================================================================
    // Lemmas: Free Range Properties
    //==================================================================================================

    /// Lemma: if a free range of size n exists starting at p, then usage <= number_of_bits - n
    proof fn lemma_free_range_implies_usage_bound(&self, p: int, n: int)
        requires
            self.inv(),
            self.has_free_range_at(p, n),
            n > 0,
        ensures
            self@.usage() <= self@.number_of_bits() - n,
    {
        // has_free_range_at(p, n) means all bits in [p, p+n) are unset.
        // That means set_bits does not contain any index in [p, p+n).
        // So set_bits ⊆ [0, num_bits) \ [p, p+n).
        // |[0, num_bits) \ [p, p+n)| = num_bits - n.
        // Therefore |set_bits| ≤ num_bits - n.
        
        let num_bits: int = self@.num_bits;
        
        // The full range [0, num_bits).
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, num_bits);
        vstd::set_lib::lemma_int_range(0, num_bits);
        
        // The free range [p, p+n).
        let free_range: Set<int> = vstd::set_lib::set_int_range(p, p + n);
        vstd::set_lib::lemma_int_range(p, p + n);
        
        // The complement: [0, num_bits) \ [p, p+n).
        let available_range: Set<int> = full_range.difference(free_range);
        
        // set_bits ⊆ available_range because:
        // - set_bits ⊆ full_range by wf()
        // - set_bits ∩ free_range = ∅ by has_free_range_at
        assert(self@.set_bits.subset_of(available_range)) by {
            assert forall|i: int| #![auto] self@.set_bits.contains(i) implies available_range.contains(i) by {
                // i is in [0, num_bits) by wf().
                assert(full_range.contains(i));
                // i is not in [p, p+n) because all_bits_unset_in_range(p, p+n).
                if p <= i && i < p + n {
                    // Contradiction: i would be in the free range but also in set_bits.
                    // has_free_range_at(p, n) implies all_bits_unset_in_range(p, p+n).
                    // all_bits_unset_in_range(p, p+n) means forall|j| p <= j < p+n ==> !is_bit_set(j).
                    // In particular, !is_bit_set(i).
                    // is_bit_set(i) = (0 <= i < num_bits) && set_bits.contains(i).
                    // Since 0 <= i < num_bits (from wf()), !is_bit_set(i) implies !set_bits.contains(i).
                    assert(self.all_bits_unset_in_range(p, p + n));
                    assert(!self.is_bit_set(i));
                    // 0 <= i < num_bits, so !is_bit_set(i) means !set_bits.contains(i).
                    assert(0 <= i && i < num_bits);
                    // Now the contradiction: i in set_bits but !set_bits.contains(i).
                }
                assert(!free_range.contains(i));
            }
        }
        
        // available_range is finite (subset of full_range).
        vstd::set_lib::lemma_set_subset_finite(full_range, available_range);
        
        // |set_bits| <= |available_range| by subset lemma.
        vstd::set_lib::lemma_len_subset(self@.set_bits, available_range);
        
        // |available_range| = |full_range| - |full_range ∩ free_range|.
        // Since free_range ⊆ full_range, |full_range ∩ free_range| = |free_range| = n.
        assert(free_range.subset_of(full_range)) by {
            assert forall|i: int| free_range.contains(i) implies full_range.contains(i) by {}
        }
        
        vstd::set_lib::lemma_len_difference(full_range, free_range);
        // available_range.len() <= full_range.len() = num_bits.
        
        // We need: available_range.len() == num_bits - n.
        // This follows from: available_range = full_range \ free_range,
        // and full_range ∩ free_range = free_range (since free_range ⊆ full_range).
        assert(full_range.intersect(free_range) =~= free_range);
        
        // full_range = available_range ∪ (full_range ∩ free_range) = available_range ∪ free_range.
        assert(full_range =~= available_range.union(free_range));
        
        // available_range and free_range are disjoint.
        assert(available_range.intersect(free_range) =~= Set::empty());
        
        // |full_range| = |available_range| + |free_range|.
        vstd::set_lib::lemma_set_disjoint_lens(available_range, free_range);
        assert(available_range.len() + free_range.len() == full_range.len());
        assert(available_range.len() == num_bits - n);
        
        // Therefore |set_bits| <= num_bits - n.
        assert(self@.usage() <= self@.number_of_bits() - n);
    }

    /// Lemma: has_free_bit implies exists_contiguous_free_range(1)
    pub proof fn lemma_has_free_bit_implies_exists_free_range_1(&self)
        requires
            self.inv(),
            self@.has_free_bit(),
        ensures
            self.exists_contiguous_free_range(1),
    {
        // has_free_bit means exists i: 0 <= i < num_bits && !set_bits.contains(i).
        let i = choose|i: int| 0 <= i < self@.number_of_bits() && !self@.set_bits.contains(i);
        assert(!self.is_bit_set(i));
        assert(self.all_bits_unset_in_range(i, i + 1));
        assert(self.has_free_range_at(i, 1));
    }

    /// Lemma: if set_bits are equal, has_free_range_at returns the same result
    pub proof fn lemma_set_bits_equal_has_free_range_at_equal(&self, other: &Self, p: int, n: int)
        requires
            self.inv(),
            other.inv(),
            self@.set_bits =~= other@.set_bits,
            self@.number_of_bits() == other@.number_of_bits(),
        ensures
            self.has_free_range_at(p, n) == other.has_free_range_at(p, n),
    {
        assert forall|i: int| 0 <= i < self@.number_of_bits() implies
            self.is_bit_set(i) == other.is_bit_set(i)
        by {
            assert(self@.set_bits.contains(i) == other@.set_bits.contains(i));
        }
    }

    /// Lemma: if set_bits are equal, exists_contiguous_free_range returns the same result
    pub proof fn lemma_set_bits_equal_exists_free_range_equal(&self, other: &Self, n: int)
        requires
            self.inv(),
            other.inv(),
            self@.set_bits =~= other@.set_bits,
            self@.number_of_bits() == other@.number_of_bits(),
        ensures
            self.exists_contiguous_free_range(n) == other.exists_contiguous_free_range(n),
    {
        assert forall|p: int| #![trigger self.has_free_range_at(p, n)]
            self.has_free_range_at(p, n) == other.has_free_range_at(p, n)
        by {
            self.lemma_set_bits_equal_has_free_range_at_equal(other, p, n);
        }

        if self.exists_contiguous_free_range(n) {
            let p = choose|p: int| #[trigger] self.has_free_range_at(p, n);
            assert(other.has_free_range_at(p, n));
        }
        if other.exists_contiguous_free_range(n) {
            let p = choose|p: int| #[trigger] other.has_free_range_at(p, n);
            assert(self.has_free_range_at(p, n));
        }
    }

    //==================================================================================================
    // Lemmas: View Synchronization
    //==================================================================================================

    /// Lemma: if bitmap is empty, no bits are set
    pub proof fn lemma_is_empty_means_no_bits_set(&self)
        requires
            self.inv(),
            self@.is_empty(),
        ensures
            forall|i: int| 0 <= i < self@.number_of_bits() ==> !self.is_bit_set(i),
    {
        // is_empty means set_bits =~= Set::empty().
    }

    /// Lemma: if bitmap is full, all bits are set
    pub proof fn lemma_is_full_means_all_bits_set(&self)
        requires
            self.inv(),
            self@.is_full(),
        ensures
            forall|i: int| 0 <= i < self@.number_of_bits() ==> self.is_bit_set(i),
    {
        // is_full means forall|i| 0 <= i < num_bits ==> set_bits.contains(i).
    }

    /// Lemma: if bitmap is full, there are no free bits
    pub proof fn lemma_is_full_implies_no_free_bit(&self)
        requires
            self.inv(),
            self@.is_full(),
        ensures
            !self@.has_free_bit(),
    {
        self.lemma_is_full_means_all_bits_set();
    }

    /// Lemma: if bitmap is not full, there exists at least one unset bit
    pub proof fn lemma_not_full_means_exists_unset_bit(&self)
        requires
            self.inv(),
            !self@.is_full(),
        ensures
            exists|i: int| 0 <= i < self@.number_of_bits() && !self.is_bit_set(i),
    {
        // not is_full means: exists|i| 0 <= i < num_bits && !set_bits.contains(i).
        assert(exists|i: int| 0 <= i < self@.num_bits && !self@.set_bits.contains(i));
        // Choose such an i.
        let i: int = choose|i: int| 0 <= i < self@.num_bits && !self@.set_bits.contains(i);
        // Prove that !self.is_bit_set(i).
        // is_bit_set(i) = (0 <= i < number_of_bits) && set_bits.contains(i)
        // Since !set_bits.contains(i), we have !is_bit_set(i).
        assert(!self.is_bit_set(i));
    }

    /// Lemma: If usage() < number_of_bits(), then the bitmap is not full.
    pub proof fn lemma_usage_less_than_capacity_means_not_full(&self)
        requires
            self.inv(),
            self@.usage() < self@.number_of_bits(),
        ensures
            !self@.is_full(),
    {
        // usage() = set_bits.len() as int.
        // number_of_bits() = num_bits.
        // set_bits ⊆ [0, num_bits) by wf().
        // |set_bits| < num_bits.
        // We need to show: exists i in [0, num_bits) such that i not in set_bits.
        
        // The full range [0, num_bits) has exactly num_bits elements.
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, self@.num_bits);
        vstd::set_lib::lemma_int_range(0, self@.num_bits);
        // full_range.len() == num_bits and full_range.finite().
        
        // set_bits ⊆ full_range.
        assert(self@.set_bits.subset_of(full_range)) by {
            assert forall|i: int| #![auto] self@.set_bits.contains(i) implies full_range.contains(i) by {
                // From wf(): set_bits.contains(i) ==> 0 <= i < num_bits.
            }
        }
        
        // By lemma_len_subset: |set_bits| <= |full_range| = num_bits.
        vstd::set_lib::lemma_len_subset(self@.set_bits, full_range);
        
        // Since |set_bits| < num_bits = |full_range|, and set_bits ⊆ full_range,
        // there must exist an element in full_range not in set_bits.
        // Proof: if set_bits = full_range, then |set_bits| = |full_range|, contradiction.
        
        // The difference full_range \ set_bits is non-empty.
        let diff: Set<int> = full_range.difference(self@.set_bits);
        
        // Prove diff is non-empty by showing its length > 0.
        // |diff| = |full_range| - |full_range ∩ set_bits| >= |full_range| - |set_bits| > 0.
        vstd::set_lib::lemma_len_difference(full_range, self@.set_bits);
        // diff.len() <= full_range.len().
        
        // Since set_bits ⊆ full_range, full_range ∩ set_bits = set_bits.
        assert(full_range.intersect(self@.set_bits) =~= self@.set_bits);
        
        // full_range = diff ∪ set_bits (disjoint union).
        assert(full_range =~= diff.union(self@.set_bits));
        
        // By disjoint union: |full_range| = |diff| + |set_bits|.
        // So |diff| = |full_range| - |set_bits| = num_bits - usage() > 0.
        vstd::set_lib::lemma_set_disjoint_lens(diff, self@.set_bits);
        assert(diff.len() > 0);
        
        // Non-empty finite set has an element.
        assert(!diff.is_empty()) by {
            vstd::set_lib::lemma_set_empty_equivalency_len(diff);
        }
        
        // Choose an element from diff.
        let i: int = diff.choose();
        // i is in full_range but not in set_bits.
        assert(0 <= i < self@.num_bits);
        assert(!self@.set_bits.contains(i));
        
        // Therefore, is_full is false (not all elements of [0, num_bits) are in set_bits).
        assert(!self@.is_full());
    }

    /// Lemma: If a specific bit is unset, then has_free_bit() is true.
    pub proof fn lemma_unset_bit_implies_has_free_bit(&self, i: int)
        requires
            self.inv(),
            0 <= i < self@.number_of_bits(),
            !self.is_bit_set(i),
        ensures
            self@.has_free_bit(),
    {
        assert(!self@.set_bits.contains(i));
    }

    /// Lemma: if all bits are set, bitmap is full
    pub proof fn lemma_all_bits_set_means_full(&self)
        requires
            self.inv(),
            forall|i: int| 0 <= i < self@.number_of_bits() ==> self.is_bit_set(i),
        ensures
            self@.is_full(),
    {
        assert forall|i: int| 0 <= i < self@.num_bits implies self@.set_bits.contains(i)
        by {
            assert(self.is_bit_set(i));
        };
    }

    //==================================================================================================
    // Lemmas: Bit-level Operations
    //==================================================================================================

    /// Lemma: Helper for proving bit operations on bytes
    proof fn lemma_bit_or_effects(old_byte: u8, bit_pos: int, new_byte: u8)
        requires
            0 <= bit_pos < 8,
            new_byte == (old_byte | (1u8 << bit_pos)),
        ensures
            (new_byte & (1u8 << bit_pos)) != 0,
            forall|other_pos: int| #![auto] 0 <= other_pos < 8 && other_pos != bit_pos ==>
                (new_byte & (1u8 << other_pos)) == (old_byte & (1u8 << other_pos)),
    {
        let shift: u8 = bit_pos as u8;
        assert((new_byte & (1u8 << shift)) != 0) by (bit_vector)
            requires
                new_byte == (old_byte | (1u8 << shift)),
                0 <= shift < 8,
        ;
        assert forall|other_pos: int| #![auto] 0 <= other_pos < 8 && other_pos != bit_pos implies
            (new_byte & (1u8 << other_pos)) == (old_byte & (1u8 << other_pos))
        by {
            let other_shift: u8 = other_pos as u8;
            assert((new_byte & (1u8 << other_shift)) == (old_byte & (1u8 << other_shift))) by (bit_vector)
                requires
                    new_byte == (old_byte | (1u8 << shift)),
                    0 <= shift < 8,
                    0 <= other_shift < 8,
                    shift != other_shift,
            ;
        }
    }

    /// Lemma: Helper for proving bit clear operations on bytes
    proof fn lemma_bit_and_not_effects(old_byte: u8, bit_pos: int, new_byte: u8)
        requires
            0 <= bit_pos < 8,
            new_byte == (old_byte & !(1u8 << bit_pos)),
        ensures
            (new_byte & (1u8 << bit_pos)) == 0,
            forall|other_pos: int| #![auto] 0 <= other_pos < 8 && other_pos != bit_pos ==>
                (new_byte & (1u8 << other_pos)) == (old_byte & (1u8 << other_pos)),
    {
        let shift: u8 = bit_pos as u8;
        assert((new_byte & (1u8 << shift)) == 0) by (bit_vector)
            requires
                new_byte == (old_byte & !(1u8 << shift)),
                0 <= shift < 8,
        ;
        assert forall|other_pos: int| #![auto] 0 <= other_pos < 8 && other_pos != bit_pos implies
            (new_byte & (1u8 << other_pos)) == (old_byte & (1u8 << other_pos))
        by {
            let other_shift: u8 = other_pos as u8;
            assert((new_byte & (1u8 << other_shift)) == (old_byte & (1u8 << other_shift))) by (bit_vector)
                requires
                    new_byte == (old_byte & !(1u8 << shift)),
                    0 <= shift < 8,
                    0 <= other_shift < 8,
                    shift != other_shift,
            ;
        }
    }

    /// Lemma: setting a byte bit reflects in set_bits
    proof fn lemma_byte_or_reflects_in_view(&self, new_self: &Self, word: int, bit: int)
        requires
            self@.number_of_bits() > 0,
            self@.number_of_bits() == self.bits@.len() * (u8::BITS as int),
            self.number_of_bits as int == self@.number_of_bits(),
            0 <= word < self.bits@.len(),
            0 <= bit < (u8::BITS as int),
            new_self.bits@.len() == self.bits@.len(),
            new_self.bits@[word] == (self.bits@[word] | (1u8 << bit)),
            forall|i: int| 0 <= i < self.bits@.len() && i != word ==> self.bits@[i] == new_self.bits@[i],
            self.number_of_bits == new_self.number_of_bits,
        ensures
            new_self@.set_bits =~= self@.set_bits.insert(word * (u8::BITS as int) + bit),
    {
        Self::lemma_bit_or_effects(self.bits@[word], bit, new_self.bits@[word]);
        let idx: int = word * (u8::BITS as int) + bit;
        
        assert forall|i: int| #![auto] new_self@.set_bits.contains(i) == self@.set_bits.insert(idx).contains(i) by {
            if i == idx {
                assert(Self::bit_at(new_self.bits@, idx));
            } else if 0 <= i < self@.number_of_bits() {
                let i_word: int = i / (u8::BITS as int);
                let i_bit: int = i % (u8::BITS as int);
                if i_word == word {
                    assert((new_self.bits@[word] & (1u8 << i_bit)) == (self.bits@[word] & (1u8 << i_bit)));
                } else {
                    assert(self.bits@[i_word] == new_self.bits@[i_word]);
                }
                assert(Self::bit_at(self.bits@, i) == Self::bit_at(new_self.bits@, i));
            }
        }
    }

    /// Lemma: clearing a byte bit reflects in set_bits
    proof fn lemma_byte_and_not_reflects_in_view(&self, new_self: &Self, word: int, bit: int)
        requires
            self.inv(),
            0 <= word < self.bits@.len(),
            0 <= bit < (u8::BITS as int),
            new_self.bits@.len() == self.bits@.len(),
            new_self.bits@[word] == (self.bits@[word] & !(1u8 << bit)),
            forall|i: int| 0 <= i < self.bits@.len() && i != word ==> self.bits@[i] == new_self.bits@[i],
            self.number_of_bits == new_self.number_of_bits,
        ensures
            new_self@.set_bits =~= self@.set_bits.remove(word * (u8::BITS as int) + bit),
    {
        Self::lemma_bit_and_not_effects(self.bits@[word], bit, new_self.bits@[word]);
        let idx: int = word * (u8::BITS as int) + bit;
        
        assert forall|i: int| #![auto] new_self@.set_bits.contains(i) == self@.set_bits.remove(idx).contains(i) by {
            if i == idx {
                assert(!Self::bit_at(new_self.bits@, idx));
            } else if 0 <= i < self@.number_of_bits() {
                let i_word: int = i / (u8::BITS as int);
                let i_bit: int = i % (u8::BITS as int);
                if i_word == word {
                    assert((new_self.bits@[word] & (1u8 << i_bit)) == (self.bits@[word] & (1u8 << i_bit)));
                } else {
                    assert(self.bits@[i_word] == new_self.bits@[i_word]);
                }
                assert(Self::bit_at(self.bits@, i) == Self::bit_at(new_self.bits@, i));
            }
        }
    }

    /// Lemma: when all raw bytes are zero, set_bits is empty
    proof fn lemma_zero_bytes_means_empty_set(&self)
        requires
            self@.number_of_bits() == self.bits@.len() * (u8::BITS as int),
            forall|i: int| 0 <= i < self.bits@.len() ==> self.bits@[i] == 0,
        ensures
            self@.set_bits =~= Set::<int>::empty(),
    {
        assert forall|i: int| !self@.set_bits.contains(i) by {
            if 0 <= i < self@.number_of_bits() {
                let byte_idx: int = i / (u8::BITS as int);
                let bit_idx: int = i % (u8::BITS as int);
                let bit_idx_u8: u8 = bit_idx as u8;
                assert((0u8 & (1u8 << bit_idx_u8)) == 0) by (bit_vector)
                    requires 0 <= bit_idx_u8 < 8;
                assert(!Self::bit_at(self.bits@, i));
            }
        }
    }

    /// Lemma: Connects closed `is_bit_set` to open `BitmapView.is_bit_set`.
    pub proof fn lemma_is_bit_set_equals_view(&self, i: int)
        requires
            self.inv(),
            0 <= i < self@.number_of_bits(),
        ensures
            self.is_bit_set(i) == self@.is_bit_set(i),
    {
        // Both reduce to self@.set_bits.contains(i).
    }

    /// Proves that number_of_bits is bounded by usize::MAX.
    pub proof fn lemma_number_of_bits_bounded(&self)
        requires
            self.inv(),
        ensures
            self@.number_of_bits() <= usize::MAX as int,
    {
        // From inv(): number_of_bits < u32::MAX, and u32::MAX <= usize::MAX.
    }

    //==========================================================================================
    // Composite lemmas (extracted from exec proof blocks for readability).
    //==========================================================================================

    /// Proves that a newly constructed bitmap (with zero-initialized bytes) satisfies inv().
    proof fn lemma_new_bitmap_inv(bmp: &Self)
        requires
            bmp@.number_of_bits() == bmp.bits@.len() * (u8::BITS as int),
            bmp@.number_of_bits() > 0,
            bmp@.number_of_bits() < u32::MAX as int,
            bmp.usage == 0,
            bmp.number_of_bits as int == bmp@.number_of_bits(),
            forall|i: int| 0 <= i < bmp.bits@.len() ==> is_zero(#[trigger] bmp.bits@[i]),
        ensures
            bmp.inv(),
            bmp@.is_empty(),
            forall|i: int| 0 <= i < bmp@.number_of_bits() ==> !bmp.is_bit_set(i),
    {
        assert forall|i: int| 0 <= i < bmp.bits@.len() implies (bmp.bits@[i] == 0) by {
            axiom_u8_zero_is_0(bmp.bits@[i]);
        };
        bmp.lemma_zero_bytes_means_empty_set();
        Self::lemma_empty_set_finite();
    }

    /// Proves inv() is preserved after setting a bit via byte OR.
    proof fn lemma_set_bit_preserves_inv(&self, new_self: &Self, word: int, bit: int, index: int)
        requires
            self.inv(),
            0 <= word < self.bits@.len(),
            0 <= bit < (u8::BITS as int),
            index == word * (u8::BITS as int) + bit,
            0 <= index < self@.number_of_bits(),
            !self@.set_bits.contains(index),
            new_self.bits@.len() == self.bits@.len(),
            new_self.bits@[word] == (self.bits@[word] | (1u8 << bit)),
            forall|i: int| 0 <= i < self.bits@.len() && i != word ==>
                self.bits@[i] == new_self.bits@[i],
            self.number_of_bits == new_self.number_of_bits,
            new_self.usage == self.usage,
        ensures
            new_self@.set_bits =~= self@.set_bits.insert(index),
            new_self@.set_bits.finite(),
            new_self@.set_bits.len() == self@.set_bits.len() + 1,
            new_self@.wf(),
            self@.set_bits.len() + 1 <= self@.number_of_bits(),
    {
        self.lemma_byte_or_reflects_in_view(new_self, word, bit);
        Self::lemma_insert_finite(self@.set_bits, index);
        Self::lemma_ext_equal_finite(new_self@.set_bits, self@.set_bits.insert(index));
        Self::lemma_insert_len(self@.set_bits, index);
        assert(new_self@.wf()) by {
            assert forall|i: int| new_self@.set_bits.contains(i) implies (0 <= i < new_self@.num_bits) by {
                if i != index {
                    assert(self@.set_bits.contains(i));
                }
            }
        }
        self.lemma_insert_preserves_usage_bound(index);
    }

    /// Proves inv() is preserved after clearing a bit via byte AND NOT.
    proof fn lemma_clear_bit_preserves_inv(&self, new_self: &Self, word: int, bit: int, index: int)
        requires
            self.inv(),
            0 <= word < self.bits@.len(),
            0 <= bit < (u8::BITS as int),
            index == word * (u8::BITS as int) + bit,
            0 <= index < self@.number_of_bits(),
            self@.set_bits.contains(index),
            new_self.bits@.len() == self.bits@.len(),
            new_self.bits@[word] == (self.bits@[word] & !(1u8 << bit)),
            forall|i: int| 0 <= i < self.bits@.len() && i != word ==>
                self.bits@[i] == new_self.bits@[i],
            self.number_of_bits == new_self.number_of_bits,
            new_self.usage == self.usage,
        ensures
            new_self@.set_bits =~= self@.set_bits.remove(index),
            new_self@.set_bits.finite(),
            new_self@.set_bits.len() == self@.set_bits.len() - 1,
            new_self@.wf(),
    {
        self.lemma_byte_and_not_reflects_in_view(new_self, word, bit);
        Self::lemma_remove_finite(self@.set_bits, index);
        Self::lemma_ext_equal_finite(new_self@.set_bits, self@.set_bits.remove(index));
        Self::lemma_remove_len(self@.set_bits, index);
        assert(new_self@.wf()) by {
            assert forall|i: int| new_self@.set_bits.contains(i) implies (0 <= i < new_self@.num_bits) by {
                assert(self@.set_bits.contains(i));
            }
        }
    }

    /// Proves no free range exists when size exceeds number_of_bits.
    proof fn lemma_no_free_range_when_size_exceeds(&self, size: int)
        requires
            self.inv(),
            size > self@.number_of_bits(),
        ensures
            !self.exists_contiguous_free_range(size),
    {
        assert forall|start: int| #![trigger self.has_free_range_at(start, size)]
            0 <= start implies !self.has_free_range_at(start, size)
        by {
            assert(start + size > self@.number_of_bits());
        }
    }

    /// Proves no free range exists when usage exceeds capacity for given size.
    proof fn lemma_no_free_range_when_usage_exceeds(&self, size: int)
        requires
            self.inv(),
            size > 0,
            size <= self@.number_of_bits(),
            self@.usage() > self@.number_of_bits() - size,
        ensures
            !self.exists_contiguous_free_range(size),
    {
        assert forall|p: int| #![trigger self.has_free_range_at(p, size)]
            0 <= p <= self@.number_of_bits() - size implies !self.has_free_range_at(p, size)
        by {
            if self.has_free_range_at(p, size) {
                self.lemma_free_range_implies_usage_bound(p, size);
            }
        }
    }

    /// Proves that when a byte is 0xFF, all 8 bits are set and no free range starts there.
    proof fn lemma_full_byte_no_free_range(&self, start: int, size: int)
        requires
            self.inv(),
            size > 0,
            start >= 0,
            start + 8 <= self@.number_of_bits(),
            start % 8 == 0,
            ({
                let word: int = start / 8;
                0 <= word < self.bits@.len() && self.bits@[word] == 0xFFu8
            }),
        ensures
            forall|i: int| start <= i < start + 8 ==> self.is_bit_set(i),
            forall|p: int| #![trigger self.has_free_range_at(p, size)]
                start <= p < start + 8 ==> !self.has_free_range_at(p, size),
    {
        assert forall|i: int| start <= i < start + 8 implies self.is_bit_set(i)
        by {
            let bit_pos: int = i % 8;
            let bit_pos_u8: u8 = bit_pos as u8;
            assert((0xFFu8 & (1u8 << bit_pos_u8)) != 0) by (bit_vector)
                requires 0 <= bit_pos_u8 < 8;
        }
        assert forall|p: int| #![trigger self.has_free_range_at(p, size)]
            start <= p < start + 8 implies !self.has_free_range_at(p, size)
        by {
            assert(self.is_bit_set(p));
        }
    }

    /// Proves that a set bit blocks any free range containing it.
    proof fn lemma_set_bit_blocks_free_range(
        &self, start_before: int, idx: int, offset: int, size: int,
    )
        requires
            self.inv(),
            0 <= start_before,
            0 <= offset < size,
            idx == start_before + offset,
            0 <= idx < self@.number_of_bits(),
            self.is_bit_set(idx),
        ensures
            forall|p: int| #![trigger self.has_free_range_at(p, size)]
                start_before <= p <= idx ==> !self.has_free_range_at(p, size),
    {
        assert forall|p: int| #![trigger self.has_free_range_at(p, size)]
            start_before <= p <= idx implies !self.has_free_range_at(p, size)
        by {
            if self.has_free_range_at(p, size) {
                assert(p <= idx);
                assert(idx - p <= offset);
                assert(offset < size);
                assert(idx < p + size);
                assert(self.is_bit_set(idx));
                assert(self.all_bits_unset_in_range(p, p + size));
                assert(!self.is_bit_set(idx));
            }
        }
    }

    /// Proves that a found free range was also free in old_self.
    proof fn lemma_free_range_was_unset_in_old(
        &self, old_self: &Self, start: int, size: int,
    )
        requires
            self.inv(),
            old_self.inv(),
            self@.set_bits =~= old_self@.set_bits,
            0 <= start,
            size > 0,
            start + size <= self@.number_of_bits(),
            forall|j: int| 0 <= j < size ==> !#[trigger] self.is_bit_set(start + j),
        ensures
            old_self.all_bits_unset_in_range(start, start + size),
    {
        assert forall|i: int| start <= i < start + size implies !#[trigger] old_self.is_bit_set(i)
        by {
            let j: int = i - start;
            assert(0 <= j && j < size);
            assert(!self.is_bit_set(start + j));
        };
    }

    /// Proves inv() after allocating a full range [start, start+size).
    proof fn lemma_alloc_range_establishes_inv(
        &self, new_self: &Self, start: int, size: int,
    )
        requires
            self.inv(),
            size > 0,
            0 <= start,
            start + size <= self@.number_of_bits(),
            self.all_bits_unset_in_range(start, start + size),
            new_self@.set_bits =~= self@.set_bits.union(
                BitmapView::range_set(start, start + size)),
            new_self@.set_bits.finite(),
            new_self.number_of_bits == self.number_of_bits,
            new_self.number_of_bits as int == new_self@.number_of_bits(),
            new_self@.number_of_bits() == new_self.bits@.len() * (u8::BITS as int),
            new_self.usage == self.usage + size,
        ensures
            new_self.inv(),
            new_self@.usage() == self@.usage() + size,
    {
        // Prove range_set is finite.
        Self::lemma_range_set_finite(start, start + size);
        // Prove union is finite.
        Self::lemma_union_finite(self@.set_bits, BitmapView::range_set(start, start + size));
        // Transfer finiteness through extensional equality.
        Self::lemma_ext_equal_finite(
            new_self@.set_bits,
            self@.set_bits.union(BitmapView::range_set(start, start + size)),
        );
        // Prove wf().
        assert(new_self@.wf()) by {
            assert forall|i: int| new_self@.set_bits.contains(i) implies (0 <= i < new_self@.num_bits) by {
                if BitmapView::range_set(start, start + size).contains(i) {
                } else {
                    assert(self@.set_bits.contains(i));
                }
            }
        }
        // Prove disjointness.
        let range: Set<int> = BitmapView::range_set(start, start + size);
        assert(self@.set_bits.disjoint(range)) by {
            assert forall|i: int| #![auto] !(self@.set_bits.contains(i) && range.contains(i)) by {
                if range.contains(i) {
                    assert(!self.is_bit_set(i));
                    assert(!self@.set_bits.contains(i));
                }
            }
        }
        // Cardinality.
        Self::lemma_disjoint_union_len(self@.set_bits, range);
        Self::lemma_range_set_len(start, start + size);
        assert(new_self@.set_bits.len() == self@.set_bits.len() + size);
        assert(new_self.usage as int == new_self@.set_bits.len());
        // Usage bound.
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, new_self@.num_bits);
        vstd::set_lib::lemma_int_range(0, new_self@.num_bits);
        assert(new_self@.set_bits.subset_of(full_range)) by {
            assert forall|i: int| #![auto] new_self@.set_bits.contains(i) implies full_range.contains(i) by {}
        }
        vstd::set_lib::lemma_len_subset(new_self@.set_bits, full_range);
    }

    /// Proves frame condition when no free range was found.
    proof fn lemma_no_range_found_frame(&self, old_self: &Self, size: int)
        requires
            self.inv(),
            old_self.inv(),
            self@.set_bits =~= old_self@.set_bits,
            self.number_of_bits == old_self.number_of_bits,
            size > 0,
            forall|p: int| #![trigger self.has_free_range_at(p, size)]
                0 <= p < self@.number_of_bits() ==> !self.has_free_range_at(p, size),
        ensures
            self@ =~= old_self@,
            !old_self.exists_contiguous_free_range(size),
    {
        assert(self@.num_bits == old_self@.num_bits);
        assert(self@ =~= old_self@);
        assert(!self.exists_contiguous_free_range(size));
        self.lemma_set_bits_equal_exists_free_range_equal(old_self, size);
    }

    /// Proves loop invariant update for a single alloc_range bit-set step.
    proof fn lemma_alloc_loop_step_inv(
        old_self: &Self, loop_old_self: &Self, new_self: &Self,
        start: int, alloc_offset: int, idx: int,
    )
        requires
            old_self.inv(),
            loop_old_self@.set_bits =~= old_self@.set_bits.union(
                BitmapView::range_set(start, start + alloc_offset)),
            loop_old_self@.set_bits.finite(),
            new_self@.set_bits =~= loop_old_self@.set_bits.insert(idx),
            idx == start + alloc_offset,
            0 <= start,
            alloc_offset >= 0,
            idx < old_self@.number_of_bits(),
            new_self@.number_of_bits() == old_self@.number_of_bits(),
            new_self.number_of_bits as int == new_self@.number_of_bits(),
        ensures
            new_self@.set_bits =~= old_self@.set_bits.union(
                BitmapView::range_set(start, start + alloc_offset + 1)),
            new_self@.wf(),
            new_self@.set_bits.finite(),
    {
        assert forall|i: int| new_self@.set_bits.contains(i) ==
            old_self@.set_bits.union(BitmapView::range_set(start, start + alloc_offset + 1)).contains(i)
        by {}
        assert(new_self@.wf()) by {
            assert forall|i: int| new_self@.set_bits.contains(i) implies (0 <= i < new_self@.num_bits) by {
                if loop_old_self@.set_bits.contains(i) {}
            }
        }
        Self::lemma_insert_finite(loop_old_self@.set_bits, idx);
    }

} // impl Bitmap

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(verus_keep_ghost)]
mod test {
    use super::*;

    /// Test: BitmapView usage calculation with Set<int>.
    proof fn test_bitmap_view_usage() {
        let view: BitmapView = BitmapView {
            num_bits: 8,
            set_bits: set![1, 3, 4],  // 3 bits set.
        };

        assert(view.number_of_bits() == 8);
        assert(view.set_bits.contains(1));
        assert(view.set_bits.contains(3));
        assert(view.set_bits.contains(4));
    }

    /// Test: Empty view has no bits set.
    proof fn test_empty_view() {
        let view: BitmapView = BitmapView {
            num_bits: 8,
            set_bits: Set::empty(),
        };

        assert(view.is_empty());
    }

    /// Test: Full view has all bits set.
    proof fn test_full_view() {
        let view: BitmapView = BitmapView {
            num_bits: 4,
            set_bits: set![0, 1, 2, 3],
        };

        // All bits should be set.
        assert(view.is_bit_set(0));
        assert(view.is_bit_set(1));
        assert(view.is_bit_set(2));
        assert(view.is_bit_set(3));
        assert(view.is_full());
    }

    /// Test: is_bit_set correctness.
    proof fn test_is_bit_set() {
        let view: BitmapView = BitmapView {
            num_bits: 8,
            set_bits: set![0, 2, 5],
        };

        assert(view.is_bit_set(0));
        assert(!view.is_bit_set(1));
        assert(view.is_bit_set(2));
        assert(!view.is_bit_set(3));
        assert(!view.is_bit_set(4));
        assert(view.is_bit_set(5));
    }

    /// Test: has_free_bit when not full.
    proof fn test_has_free_bit() {
        let view: BitmapView = BitmapView {
            num_bits: 4,
            set_bits: set![0, 2],  // 2 bits set, 2 free.
        };

        // Bits 1 and 3 are free.
        assert(!view.is_bit_set(1));
        assert(!view.is_bit_set(3));
        // Witness for has_free_bit.
        assert(0 <= 1 < view.number_of_bits() && !view.set_bits.contains(1));
        assert(view.has_free_bit());
    }

    /// Test: wf (well-formedness) property.
    proof fn test_wf() {
        let view: BitmapView = BitmapView {
            num_bits: 8,
            set_bits: set![0, 3, 7],  // All indices in [0, 8).
        };

        // All set bits are within bounds.
        assert(view.wf());
    }
}

} // verus!
