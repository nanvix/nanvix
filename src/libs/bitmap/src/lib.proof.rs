// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Bitmap - Proofs
//
// This file contains lemmas and proof functions for Bitmap.

verus! {

impl Bitmap {
    //==================================================================================================
    // Lemmas: Layout
    //==================================================================================================
    /// Proves that `len` bytes fit within isize::MAX for RawArray allocation.
    proof fn lemma_u8_array_len_fits_isize(len: usize)
        requires
            len <= u32::MAX as usize / (u8::BITS as usize),
        ensures
            len * vstd::layout::size_of::<u8>() + vstd::layout::align_of::<u8>() - 1
                <= isize::MAX as usize,
    {
        broadcast use vstd::layout::layout_of_primitives, vstd::layout::align_of_u8;
    }

    //==================================================================================================
    // Lemmas: Bit-level Operations
    //==================================================================================================

    /// Bit OR sets the target bit and preserves all other bits.
    proof fn lemma_bit_or_effects(old_byte: u8, bit_pos: int, new_byte: u8)
        requires
            0 <= bit_pos < 8,
            new_byte == (old_byte | (1u8 << bit_pos)),
        ensures
            (new_byte & (1u8 << bit_pos)) != 0,
            forall|other_pos: int|
                #![auto]
                0 <= other_pos < 8 && other_pos != bit_pos ==> (new_byte & (1u8 << other_pos)) == (
                old_byte & (1u8 << other_pos)),
    {
        let shift: u8 = bit_pos as u8;
        assert((new_byte & (1u8 << shift)) != 0) by (bit_vector)
            requires
                new_byte == (old_byte | (1u8 << shift)),
                0 <= shift < 8,
        ;
        assert forall|other_pos: int| #![auto] 0 <= other_pos < 8 && other_pos != bit_pos implies (
        new_byte & (1u8 << other_pos)) == (old_byte & (1u8 << other_pos)) by {
            let other_shift: u8 = other_pos as u8;
            assert((new_byte & (1u8 << other_shift)) == (old_byte & (1u8 << other_shift)))
                by (bit_vector)
                requires
                    new_byte == (old_byte | (1u8 << shift)),
                    0 <= shift < 8,
                    0 <= other_shift < 8,
                    shift != other_shift,
            ;
        }
    }

    /// Bit AND NOT clears the target bit and preserves all other bits.
    proof fn lemma_bit_and_not_effects(old_byte: u8, bit_pos: int, new_byte: u8)
        requires
            0 <= bit_pos < 8,
            new_byte == (old_byte & !(1u8 << bit_pos)),
        ensures
            (new_byte & (1u8 << bit_pos)) == 0,
            forall|other_pos: int|
                #![auto]
                0 <= other_pos < 8 && other_pos != bit_pos ==> (new_byte & (1u8 << other_pos)) == (
                old_byte & (1u8 << other_pos)),
    {
        let shift: u8 = bit_pos as u8;
        assert((new_byte & (1u8 << shift)) == 0) by (bit_vector)
            requires
                new_byte == (old_byte & !(1u8 << shift)),
                0 <= shift < 8,
        ;
        assert forall|other_pos: int| #![auto] 0 <= other_pos < 8 && other_pos != bit_pos implies (
        new_byte & (1u8 << other_pos)) == (old_byte & (1u8 << other_pos)) by {
            let other_shift: u8 = other_pos as u8;
            assert((new_byte & (1u8 << other_shift)) == (old_byte & (1u8 << other_shift)))
                by (bit_vector)
                requires
                    new_byte == (old_byte & !(1u8 << shift)),
                    0 <= shift < 8,
                    0 <= other_shift < 8,
                    shift != other_shift,
            ;
        }
    }

    /// Setting a byte bit reflects in set_bits.
    proof fn lemma_byte_or_reflects_in_view(&self, new_self: &Self, word: int, bit: int)
        requires
            self@.num_bits > 0,
            self@.num_bits == self.bits@.len() * (u8::BITS as int),
            self.number_of_bits as int == self@.num_bits,
            0 <= word < self.bits@.len(),
            0 <= bit < (u8::BITS as int),
            new_self.bits@.len() == self.bits@.len(),
            new_self.bits@[word] == (self.bits@[word] | (1u8 << bit)),
            forall|i: int|
                0 <= i < self.bits@.len() && i != word ==> self.bits@[i] == new_self.bits@[i],
            self.number_of_bits == new_self.number_of_bits,
        ensures
            new_self@.set_bits =~= self@.set_bits.insert(word * (u8::BITS as int) + bit),
    {
        Self::lemma_bit_or_effects(self.bits@[word], bit, new_self.bits@[word]);
        let idx: int = word * (u8::BITS as int) + bit;
        assert forall|i: int|
            #![auto]
            new_self@.set_bits.contains(i) == self@.set_bits.insert(idx).contains(i) by {
            if i != idx && 0 <= i < self@.num_bits {
                let i_word: int = i / (u8::BITS as int);
                if i_word == word {
                    let i_bit: int = i % (u8::BITS as int);
                    assert((new_self.bits@[word] & (1u8 << i_bit)) == (self.bits@[word] & (1u8
                        << i_bit)));
                }
            }
        }
    }

    /// Clearing a byte bit reflects in set_bits.
    proof fn lemma_byte_and_not_reflects_in_view(&self, new_self: &Self, word: int, bit: int)
        requires
            self.inv(),
            0 <= word < self.bits@.len(),
            0 <= bit < (u8::BITS as int),
            new_self.bits@.len() == self.bits@.len(),
            new_self.bits@[word] == (self.bits@[word] & !(1u8 << bit)),
            forall|i: int|
                0 <= i < self.bits@.len() && i != word ==> self.bits@[i] == new_self.bits@[i],
            self.number_of_bits == new_self.number_of_bits,
        ensures
            new_self@.set_bits =~= self@.set_bits.remove(word * (u8::BITS as int) + bit),
    {
        Self::lemma_bit_and_not_effects(self.bits@[word], bit, new_self.bits@[word]);
        let idx: int = word * (u8::BITS as int) + bit;
        assert forall|i: int|
            #![auto]
            new_self@.set_bits.contains(i) == self@.set_bits.remove(idx).contains(i) by {
            if i != idx && 0 <= i < self@.num_bits {
                let i_word: int = i / (u8::BITS as int);
                if i_word == word {
                    let i_bit: int = i % (u8::BITS as int);
                    assert((new_self.bits@[word] & (1u8 << i_bit)) == (self.bits@[word] & (1u8
                        << i_bit)));
                }
            }
        }
    }

    /// When all raw bytes are zero, set_bits is empty.
    proof fn lemma_zero_bytes_means_empty_set(&self)
        requires
            self@.num_bits == self.bits@.len() * (u8::BITS as int),
            forall|i: int| 0 <= i < self.bits@.len() ==> self.bits@[i] == 0,
        ensures
            self@.set_bits == Set::<int>::empty(),
    {
        assert forall|i: int| !self@.set_bits.contains(i) by {
            if 0 <= i < self@.num_bits {
                let bit_idx_u8: u8 = (i % (u8::BITS as int)) as u8;
                assert((0u8 & (1u8 << bit_idx_u8)) == 0) by (bit_vector)
                    requires
                        0 <= bit_idx_u8 < 8,
                ;
            }
        }
    }

    /// number_of_bits is bounded by usize::MAX.
    pub proof fn lemma_number_of_bits_bounded(&self)
        requires
            self.inv(),
        ensures
            self@.num_bits <= usize::MAX as int,
    {
    }

    //==========================================================================================
    // Composite Lemmas
    //==========================================================================================

    /// Proves that a newly constructed bitmap (with zero-initialized bytes) satisfies inv().
    proof fn lemma_new_bitmap_inv(bmp: &Self)
        requires
            bmp.bits.inv(),
            bmp@.num_bits == bmp.bits@.len() * (u8::BITS as int),
            bmp@.num_bits > 0,
            bmp@.num_bits < u32::MAX as int,
            bmp.usage == 0,
            bmp.next_free == 0,
            bmp.number_of_bits as int == bmp@.num_bits,
            forall|i: int| 0 <= i < bmp.bits@.len() ==> is_zero(#[trigger] bmp.bits@[i]),
        ensures
            bmp.inv(),
            bmp@.is_empty(),
    {
        assert forall|i: int| 0 <= i < bmp.bits@.len() implies (bmp.bits@[i] == 0) by {
            axiom_u8_zero_is_0(bmp.bits@[i]);
        };
        bmp.lemma_zero_bytes_means_empty_set();
    }

    /// Proves inv() is preserved after setting a bit via byte OR.
    proof fn lemma_set_bit_preserves_inv(&self, new_self: &Self, word: int, bit: int, index: int)
        requires
            self.inv(),
            0 <= word < self.bits@.len(),
            0 <= bit < (u8::BITS as int),
            index == word * (u8::BITS as int) + bit,
            0 <= index < self@.num_bits,
            !self@.set_bits.contains(index),
            new_self.bits@.len() == self.bits@.len(),
            new_self.bits@[word] == (self.bits@[word] | (1u8 << bit)),
            forall|i: int|
                0 <= i < self.bits@.len() && i != word ==> self.bits@[i] == new_self.bits@[i],
            self.number_of_bits == new_self.number_of_bits,
            new_self.usage == self.usage,
        ensures
            new_self@.set_bits =~= self@.set_bits.insert(index),
            new_self@.set_bits.finite(),
            new_self@.set_bits.len() == self@.set_bits.len() + 1,
            new_self@.wf(),
            self@.set_bits.len() + 1 <= self@.num_bits,
    {
        self.lemma_byte_or_reflects_in_view(new_self, word, bit);
        assert(new_self@.wf()) by {
            assert forall|i: int| new_self@.set_bits.contains(i) implies (0 <= i
                < new_self@.num_bits) by {
                if i != index {
                    assert(self@.set_bits.contains(i));
                }
            }
        }
        self@.lemma_insert_preserves_usage_bound(index);
    }

    /// Proves inv() is preserved after clearing a bit via byte AND NOT.
    proof fn lemma_clear_bit_preserves_inv(&self, new_self: &Self, word: int, bit: int, index: int)
        requires
            self.inv(),
            0 <= word < self.bits@.len(),
            0 <= bit < (u8::BITS as int),
            index == word * (u8::BITS as int) + bit,
            0 <= index < self@.num_bits,
            self@.set_bits.contains(index),
            new_self.bits@.len() == self.bits@.len(),
            new_self.bits@[word] == (self.bits@[word] & !(1u8 << bit)),
            forall|i: int|
                0 <= i < self.bits@.len() && i != word ==> self.bits@[i] == new_self.bits@[i],
            self.number_of_bits == new_self.number_of_bits,
            new_self.usage == self.usage,
        ensures
            new_self@.set_bits =~= self@.set_bits.remove(index),
            new_self@.set_bits.finite(),
            new_self@.set_bits.len() == self@.set_bits.len() - 1,
            new_self@.wf(),
    {
        self.lemma_byte_and_not_reflects_in_view(new_self, word, bit);
        assert(new_self@.wf()) by {
            assert forall|i: int| new_self@.set_bits.contains(i) implies (0 <= i
                < new_self@.num_bits) by {
                assert(self@.set_bits.contains(i));
            }
        }
    }

    /// Proves no free range exists when size exceeds number_of_bits.
    proof fn lemma_no_free_range_when_size_exceeds(&self, size: int)
        requires
            self.inv(),
            size > self@.num_bits,
        ensures
            !self@.exists_contiguous_free_range(size),
    {
        assert forall|start: int|
            #![trigger self@.has_free_range_at(start, size)]
            0 <= start implies !self@.has_free_range_at(start, size) by {}
    }

    /// Proves no free range exists when usage exceeds capacity for given size.
    proof fn lemma_no_free_range_when_usage_exceeds(&self, size: int)
        requires
            self.inv(),
            size > 0,
            size <= self@.num_bits,
            self@.usage() > self@.num_bits - size,
        ensures
            !self@.exists_contiguous_free_range(size),
    {
        assert forall|p: int|
            #![trigger self@.has_free_range_at(p, size)]
            0 <= p <= self@.num_bits - size implies !self@.has_free_range_at(p, size) by {
            if self@.has_free_range_at(p, size) {
                self@.lemma_free_range_implies_usage_bound(p, size);
            }
        }
    }

    /// Proves that when a byte is 0xFF, all 8 bits are set and no free range starts there.
    proof fn lemma_full_byte_no_free_range(&self, start: int, size: int)
        requires
            self.inv(),
            size > 0,
            start >= 0,
            start + 8 <= self@.num_bits,
            start % 8 == 0,
            ({
                let word: int = start / 8;
                0 <= word < self.bits@.len() && self.bits@[word] == 0xFFu8
            }),
        ensures
            forall|i: int| start <= i < start + 8 ==> self@.is_bit_set(i),
            forall|p: int|
                #![trigger self@.has_free_range_at(p, size)]
                start <= p < start + 8 ==> !self@.has_free_range_at(p, size),
    {
        assert forall|i: int| start <= i < start + 8 implies Self::bit_at(self.bits@, i) by {
            let bit_pos_u8: u8 = (i % 8) as u8;
            assert((0xFFu8 & (1u8 << bit_pos_u8)) != 0) by (bit_vector)
                requires
                    0 <= bit_pos_u8 < 8,
            ;
        }
        assert forall|p: int|
            #![trigger self@.has_free_range_at(p, size)]
            start <= p < start + 8 implies !self@.has_free_range_at(p, size) by {
            assert(self@.is_bit_set(p));
        }
    }

    /// Proves that a set bit blocks any free range containing it.
    proof fn lemma_set_bit_blocks_free_range(
        &self,
        start_before: usize,
        idx: usize,
        offset: usize,
        size: usize,
    )
        requires
            self.inv(),
            0 <= start_before,
            0 <= offset < size,
            idx == start_before + offset,
            0 <= idx < self@.num_bits,
            self@.is_bit_set(idx as int),
        ensures
            forall|p: int|
                #![trigger self@.has_free_range_at(p, size as int)]
                start_before <= p <= idx ==> !self@.has_free_range_at(p, size as int),
    {
        assert forall|p: int|
            #![trigger self@.has_free_range_at(p, size as int)]
            start_before <= p <= idx implies !self@.has_free_range_at(p, size as int) by {
            if self@.has_free_range_at(p, size as int) {
                assert(!self@.is_bit_set(idx as int));
            }
        }
    }

    /// Proves that a found free range was also free in old_self.
    proof fn lemma_free_range_was_unset_in_old(&self, old_self: &Self, start: usize, size: usize)
        requires
            self.inv(),
            old_self.inv(),
            self@.set_bits =~= old_self@.set_bits,
            0 <= start,
            size > 0,
            start + size <= self@.num_bits,
            forall|j: int| 0 <= j < size ==> !#[trigger] self@.is_bit_set(start + j),
        ensures
            old_self@.all_bits_unset_in_range(start as int, start + size),
    {
        assert forall|i: int| start <= i < start + size implies !#[trigger] old_self@.is_bit_set(
            i,
        ) by {
            assert(!self@.is_bit_set(start + (i - start)));
        };
    }

    /// Proves inv() after allocating a full range [start, start+size).
    proof fn lemma_alloc_range_establishes_inv(&self, old_self: &Self, start: usize, size: usize)
        requires
            old_self.inv(),
            size > 0,
            0 <= start,
            start + size <= old_self@.num_bits,
            old_self@.all_bits_unset_in_range(start as int, start + size),
            self.bits.inv(),
            self@.set_bits =~= old_self@.set_bits.union(BitmapView::range_set(start as int, start + size)),
            self@.set_bits.finite(),
            self.number_of_bits == old_self.number_of_bits,
            self.number_of_bits as int == self@.num_bits,
            self@.num_bits == self.bits@.len() * (u8::BITS as int),
            self.usage == old_self.usage + size,
            self.next_free as int <= self@.num_bits,
        ensures
            self.inv(),
            self@.usage() == old_self@.usage() + size,
    {
        BitmapView::lemma_range_set_finite(start as int, start + size);
        let range: Set<int> = BitmapView::range_set(start as int, start + size);
        assert(self@.wf()) by {
            assert forall|i: int| self@.set_bits.contains(i) implies (0 <= i
                < self@.num_bits) by {
                if !range.contains(i) {
                    assert(old_self@.set_bits.contains(i));
                }
            }
        }
        assert(old_self@.set_bits.disjoint(range)) by {
            assert forall|i: int| #![auto] !(old_self@.set_bits.contains(i) && range.contains(i)) by {
                if range.contains(i) {
                    assert(!old_self@.is_bit_set(i));
                }
            }
        }
        vstd::set_lib::lemma_set_disjoint_lens(old_self@.set_bits, range);
        BitmapView::lemma_range_set_len(start as int, start + size);
        let full_range: Set<int> = vstd::set_lib::set_int_range(0, self@.num_bits);
        vstd::set_lib::lemma_int_range(0, self@.num_bits);
        assert(self@.set_bits.subset_of(full_range)) by {
            assert forall|i: int|
                #![auto]
                self@.set_bits.contains(i) implies full_range.contains(i) by {}
        }
        vstd::set_lib::lemma_len_subset(self@.set_bits, full_range);
    }

    /// Proves frame condition when no free range was found.
    proof fn lemma_no_range_found_frame(&self, old_self: &Self, size: int)
        requires
            self.inv(),
            old_self.inv(),
            self@.set_bits =~= old_self@.set_bits,
            self.number_of_bits == old_self.number_of_bits,
            size > 0,
            forall|p: int|
                #![trigger self@.has_free_range_at(p, size)]
                0 <= p < self@.num_bits ==> !self@.has_free_range_at(p, size),
        ensures
            self@ =~= old_self@,
            !old_self@.exists_contiguous_free_range(size),
    {
        self@.lemma_set_bits_equal_exists_free_range_equal(&(old_self@), size);
    }

    /// Proves loop invariant update for a single alloc_range bit-set step.
    proof fn lemma_alloc_loop_step_inv(
        self: &Self,
        old_self: &Self,
        loop_old_self: &Self,
        start: usize,
        alloc_offset: usize,
        idx: usize,
    )
        requires
            old_self.inv(),
            loop_old_self@.set_bits =~= old_self@.set_bits.union(
                BitmapView::range_set(start as int, start + alloc_offset),
            ),
            loop_old_self@.set_bits.finite(),
            self@.set_bits =~= loop_old_self@.set_bits.insert(idx as int),
            idx == start + alloc_offset,
            0 <= start,
            alloc_offset >= 0,
            idx < old_self@.num_bits,
            self@.num_bits == old_self@.num_bits,
            self.number_of_bits as int == self@.num_bits,
        ensures
            self@.set_bits =~= old_self@.set_bits.union(
                BitmapView::range_set(start as int, start + alloc_offset + 1),
            ),
            self@.wf(),
            self@.set_bits.finite(),
    {
        assert forall|i: int|
            self@.set_bits.contains(i) == old_self@.set_bits.union(
                BitmapView::range_set(start as int, start + alloc_offset + 1),
            ).contains(i) by {}
        assert(self@.wf()) by {
            assert forall|i: int| self@.set_bits.contains(i) implies (0 <= i
                < self@.num_bits) by {}
        }
    }

    // Invariant for the first (outermost) loop in alloc_range
    spec fn alloc_range_first_loop_invariant(
        self: &Self,
        old_self: &Self,
        size: usize,
        start: usize,
        initial_start: usize,
        wrapped: bool,
        done: bool
    ) -> bool
    {
        &&& self.inv()
        &&& old_self.inv()
        &&& size > 0
        &&& size <= self.number_of_bits
        &&& start <= self.number_of_bits
        &&& self@.set_bits =~= old_self@.set_bits
        &&& self.usage <= self.number_of_bits - size
        &&& self.number_of_bits == old_self.number_of_bits
        &&& initial_start as int <= self@.num_bits
        // Wrap-around state consistency.
        &&& !wrapped ==> start >= initial_start
        &&& wrapped ==> initial_start > 0
        // Checked positions.
        &&& !wrapped ==> forall|p: int|
               #![trigger self@.has_free_range_at(p, size as int)]
               initial_start as int <= p < start as int ==> !self@.has_free_range_at(
                   p,
                   size as int,
               )
        &&& wrapped ==> forall|p: int|
               #![trigger self@.has_free_range_at(p, size as int)]
               initial_start as int <= p < self@.num_bits ==> !self@.has_free_range_at(
                   p,
                   size as int,
               )
        &&& wrapped ==> forall|p: int|
               #![trigger self@.has_free_range_at(p, size as int)]
               0 <= p < start as int ==> !self@.has_free_range_at(p, size as int)
        // When done, all positions have been checked.
        &&& done ==> forall|p: int|
               #![trigger self@.has_free_range_at(p, size as int)]
               0 <= p < self@.num_bits ==> !self@.has_free_range_at(p, size as int)
    }

    // Invariant for the second loop in alloc_range
    spec fn alloc_range_second_loop_invariant(
        self: &Self,
        old_self: &Self,
        size: usize,
        start: usize,
        initial_start: usize,
        offset: usize,
        wrapped: bool,
        free: bool,
        checked_before: int,
        start_before_inner: usize,
    ) -> bool
    {
        &&& self.inv()
        &&& old_self.inv()
        &&& 0 < size <= self.number_of_bits
        &&& offset <= size
        &&& start_before_inner <= self.number_of_bits - size
        &&& self@.set_bits =~= old_self@.set_bits
        &&& checked_before == start_before_inner as int
        // Positions before start_before_inner are already checked.
        &&& forall|p: int|
              #![trigger self@.has_free_range_at(p, size as int)]
              (!wrapped ==> initial_start as int <= p < checked_before
                  ==> !self@.has_free_range_at(p, size as int))
        &&& forall|p: int|
                  #![trigger self@.has_free_range_at(p, size as int)]
                  (wrapped ==> 0 <= p < checked_before ==> !self@.has_free_range_at(
                      p,
                      size as int,
                  ))
        &&& free ==> forall|i: int|
                  0 <= i < offset ==> !#[trigger] self@.is_bit_set(
                      (start_before_inner + i) as int,
                  )
    }

    // Loop postcondition for the second loop in alloc_range
    spec fn alloc_range_second_loop_ensures(
        self: &Self,
        size: usize,
        start: usize,
        initial_start: usize,
        wrapped: bool,
        free: bool,
        start_before_inner: usize,
    ) -> bool
    {
        &&& start <= self.number_of_bits
        &&& free ==> start == start_before_inner && start <= self.number_of_bits - size
              && forall|i: int|
              0 <= i < size ==> !#[trigger] self@.is_bit_set((start + i) as int)
        &&& !free ==> start > start_before_inner
        &&& !free ==> forall|p: int|
              #![trigger self@.has_free_range_at(p, size as int)]
              (!wrapped ==> initial_start as int <= p < start as int
                  ==> !self@.has_free_range_at(p, size as int))
        &&& !free ==> forall|p: int|
              #![trigger self@.has_free_range_at(p, size as int)]
              (wrapped ==> 0 <= p < start as int ==> !self@.has_free_range_at(
                  p,
                  size as int,
              ))
    }

    // Invariant for the third loop in alloc_range
    spec fn alloc_range_third_loop_invariant(
        self: &Self,
        old_self: &Self,
        pre_alloc_self: Self,
        size: usize,
        start: usize,
        alloc_offset: usize,
    ) -> bool
    {
        &&& self.bits.inv()
        &&& self.bits@.len() == pre_alloc_self.bits@.len()
        &&& self.bits@.len() == old_self.bits@.len()
        &&& self@.num_bits > 0
        &&& self@.num_bits == self.bits@.len() * (u8::BITS as int)
        &&& self.number_of_bits == pre_alloc_self.number_of_bits
        &&& self.number_of_bits as int == self@.num_bits
        &&& self.usage == pre_alloc_self.usage
        &&& old_self.inv()
        &&& pre_alloc_self.inv()
        &&& 0 < size <= self.number_of_bits
        &&& start <= self.number_of_bits - size
        &&& alloc_offset <= size
        &&& forall|i: int|
               0 <= i < alloc_offset ==> #[trigger] self@.is_bit_set(
                   (start + i) as int,
               )
        &&& forall|i: int|
               (0 <= i < self@.num_bits && (i < start as int || i >= (start
                   + alloc_offset) as int)) ==> #[trigger] self@.is_bit_set(i)
                   == #[trigger] old_self@.is_bit_set(i)
        &&& self@.set_bits =~= old_self@.set_bits.union(
               BitmapView::range_set(
                   start as int,
                   start as int + (alloc_offset as int),
               ),
           )
        &&& self@.set_bits.finite()
        &&& old_self@.all_bits_unset_in_range(
               start as int,
               start as int + (size as int),
           )
    }

    /// Proves that positions in `[lower, N)` have no free range of given size
    /// when all positions in `[lower, checked)` were already checked and the
    /// remaining positions have `p + size > N`.
    proof fn lemma_phase1_complete_no_free_range(&self, initial_start: usize, start: usize, size: usize)
        requires
            self.inv(),
            size > 0,
            initial_start >= 0,
            initial_start <= self@.num_bits,
            start >= initial_start,
            start <= self@.num_bits,
            start > self@.num_bits - size,
            forall|p: int|
                #![trigger self@.has_free_range_at(p, size as int)]
                initial_start <= p < start ==> !self@.has_free_range_at(p, size as int),
        ensures
            forall|p: int|
                #![trigger self@.has_free_range_at(p, size as int)]
                initial_start <= p < self@.num_bits ==> !self@.has_free_range_at(p, size as int),
    {
        assert forall|p: int|
            #![trigger self@.has_free_range_at(p, size as int)]
            initial_start <= p < self@.num_bits implies !self@.has_free_range_at(p, size as int) by {}
    }

    /// Proves that all positions in `[0, N)` have no free range when both
    /// phases have been checked.
    proof fn lemma_all_positions_no_free_range(
        &self,
        initial_start: usize,
        start: usize,
        size: usize,
        wrapped: bool,
    )
        requires
            self.inv(),
            size > 0,
            start <= self@.num_bits,
            start > self@.num_bits - size || (wrapped && start >= initial_start),
            initial_start >= 0,
            initial_start <= self@.num_bits,
            // Phase 1 covered [initial_start, N).
            wrapped ==> forall|p: int|
                #![trigger self@.has_free_range_at(p, size as int)]
                initial_start <= p < self@.num_bits ==> !self@.has_free_range_at(p, size as int),
            // Phase 2 covered [0, start).
            wrapped ==> forall|p: int|
                #![trigger self@.has_free_range_at(p, size as int)]
                0 <= p < start ==> !self@.has_free_range_at(p, size as int),
            // If not wrapped: phase 1 covered [initial_start, start), and initial_start == 0.
            !wrapped ==> initial_start == 0,
            !wrapped ==> forall|p: int|
                #![trigger self@.has_free_range_at(p, size as int)]
                0 <= p < start ==> !self@.has_free_range_at(p, size as int),
        ensures
            forall|p: int|
                #![trigger self@.has_free_range_at(p, size as int)]
                0 <= p < self@.num_bits ==> !self@.has_free_range_at(p, size as int),
    {
        assert forall|p: int|
            #![trigger self@.has_free_range_at(p, size as int)]
            0 <= p < self@.num_bits implies !self@.has_free_range_at(p, size as int) by {
            if wrapped && p >= start && p < initial_start {
                assert(p + size > self@.num_bits);
            }
        }
    }

} // impl Bitmap

} // end verus!
