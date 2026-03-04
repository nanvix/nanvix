// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Proofs and lemmas.

verus! {

impl Slab {
    //==============================================================================================

    /// Loop invariant for the index initialization loop in `from_raw_parts`.
    spec fn from_raw_parts_init_loop_invariant(
        index: Bitmap,
        i: usize,
        num_index_blocks: usize,
        num_data_blocks: usize,
        total_num_blocks: usize,
        block_size: usize,
        data_addr: *mut u8,
    ) -> bool {
        &&& index.inv()
        &&& i <= num_index_blocks
        &&& num_index_blocks < total_num_blocks
        &&& num_index_blocks > 0
        &&& num_data_blocks > 0
        &&& block_size > 0
        &&& data_addr as int > 0
        &&& num_index_blocks + num_data_blocks == total_num_blocks
        &&& index@.num_bits == total_num_blocks as int
        &&& num_index_blocks as int + num_data_blocks as int == index@.num_bits
        &&& forall|j: int|
            #![trigger index@.set_bits.contains(j)]
            0 <= j < i as int ==> index@.set_bits.contains(j)
        &&& forall|j: int|
            #![trigger index@.set_bits.contains(j)]
            i as int <= j < index@.num_bits ==> !index@.set_bits.contains(j)
    }

    //==============================================================================================
    /// Lemma: Prove that a Slab satisfies the invariant given its components satisfy the conditions.
    proof fn lemma_inv_from_components(slab: &Slab)
        requires
            slab.index.inv(),
            slab.block_size > 0,
            slab.num_data_blocks > 0,
            slab.num_index_blocks > 0,
            slab.num_index_blocks + slab.num_data_blocks == slab.index@.num_bits,
            forall|i: int|
                #![trigger slab.index@.set_bits.contains(i)]
                0 <= i < slab.num_index_blocks as int ==> slab.index@.set_bits.contains(i),
            slab.data_addr as int > 0,
            (slab.num_data_blocks as int) * (slab.block_size as int) <= usize::MAX as int,
            (slab.data_addr as int) + (slab.num_data_blocks as int) * (slab.block_size as int)
                <= usize::MAX as int,
            slab.data_addr as int >= slab.num_index_blocks as int * slab.block_size as int,
            is_pow2(slab.block_size as int),
            slab.data_addr as int % slab.block_size as int == 0,
        ensures
            slab.inv(),
    {
    }

    /// Lemma: Reveal the relationship between slab view and slab fields.
    proof fn lemma_view_fields(slab: &Slab)
        requires
            slab.inv(),
        ensures
            slab@.block_size == slab.block_size as int,
            slab@.data_addr == slab.data_addr as int,
            slab@.num_data_blocks == slab.num_data_blocks as int,
    {
    }

    /// Lemma: If no block is allocated, the slab is empty.
    /// Bridges `forall|i| !is_allocated(i)` to `is_empty()`.
    ///
    /// # Proof Strategy
    ///
    /// We prove that allocated_blocks == empty set by showing no element can be in it.
    /// - For i in [0, num_data_blocks): !is_allocated(i) by precondition.
    /// - For i outside this range: !is_allocated(i) by allocated_blocks_in_range (from inv).
    /// Therefore, forall i, !allocated_blocks.contains(i), so allocated_blocks =~= Set::empty().
    pub proof fn lemma_no_allocated_implies_empty(slab: &Slab)
        requires
            slab.inv(),
            forall|i: int| 0 <= i < slab@.num_data_blocks ==> !slab@.is_allocated(i),
        ensures
            slab@.is_empty(),
    {
        // Proof: All data blocks unset means no block in allocated_blocks.
        assert(slab@.allocated_blocks =~= Set::<int>::empty()) by {
            // For any i, show !allocated_blocks.contains(i).
            assert forall|i: int| !slab@.allocated_blocks.contains(i) by {
                if 0 <= i < slab@.num_data_blocks {
                    assert(!slab@.is_allocated(i));
                } else {
                    assert(slab@.allocated_blocks_in_range());
                    assert(!slab@.is_allocated(i));
                }
            }
        }
    }

    /// Lemma: Reveal that a newly created slab with no data blocks allocated has no allocated blocks.
    proof fn lemma_new_slab_is_empty(slab: &Slab)
        requires
            slab.inv(),
            forall|i: int|
                slab.num_index_blocks as int <= i < (slab.num_index_blocks
                    + slab.num_data_blocks) as int ==> !slab.index@.is_bit_set(i),
        ensures
            forall|i: int| 0 <= i < slab@.num_data_blocks ==> !slab@.is_allocated(i),
    {
        assert forall|i: int| 0 <= i < slab@.num_data_blocks implies !slab@.is_allocated(i) by {
            let bitmap_idx = slab.num_index_blocks as int + i;
            assert(!slab.index@.is_bit_set(bitmap_idx));
        }
    }

    /// Lemma: Slab invariant implies Bitmap invariant.
    proof fn lemma_slab_inv_implies_bitmap_inv(&self)
        requires
            self.inv(),
        ensures
            self.index.inv(),
            self.index@.num_bits <= (usize::MAX as int),
    {
        self.index.lemma_number_of_bits_bounded();
    }

    //==============================================================================================
    /// Helper lemma: allocated_blocks is a subset of set_int_range(0, num_data_blocks).
    /// This follows from allocated_blocks_in_range: forall|i| is_allocated(i) ==> 0 <= i < num_data_blocks.
    proof fn lemma_allocated_blocks_subset_of_range(&self)
        requires
            self.inv(),
        ensures
            self@.allocated_blocks.subset_of(set_int_range(0, self@.num_data_blocks)),
    {
    }

    /// Helper lemma: allocated_blocks is finite.
    /// Since allocated_blocks is a subset of set_int_range(0, num_data_blocks), and that's finite,
    /// allocated_blocks is also finite.
    proof fn lemma_allocated_blocks_finite(&self)
        requires
            self.inv(),
        ensures
            self@.allocated_blocks.finite(),
    {
        let num_data: int = self@.num_data_blocks;
        // Now prove allocated_blocks == set_int_range(0, num_data).
        let full_range: Set<int> = set_int_range(0, num_data);

        // Prove full_range is finite.
        lemma_int_range(0, num_data);
        assert(full_range.finite());

        // Use lemma_allocated_blocks_subset_of_range to prove the subset property.
        self.lemma_allocated_blocks_subset_of_range();
        assert(self@.allocated_blocks.subset_of(full_range));

        lemma_set_subset_finite(full_range, self@.allocated_blocks);
    }

    /// Lemma (Liveness): If slab can_allocate(), then bitmap has_free_bit().
    /// This is the key lemma connecting slab liveness to bitmap liveness.
    ///
    /// Proof sketch:
    /// - can_allocate() means free() > 0
    /// - free() = capacity - used = num_data_blocks - |allocated_blocks|
    /// - If free() > 0, then |allocated_blocks| < num_data_blocks
    /// - allocated_blocks = { i | 0 <= i < num_data_blocks && is_bit_set(num_index_blocks + i) }
    /// - If |allocated_blocks| < num_data_blocks, there exists some data index j not in allocated_blocks
    /// - That means is_bit_set(num_index_blocks + j) is false
    /// - Therefore, there's an unset bit in the bitmap => has_free_bit()
    ///
    /// Proves that if the slab can allocate, then the bitmap has a free bit.
    /// This connects liveness (can_allocate) to the concrete bitmap state.
    proof fn lemma_can_allocate_implies_bitmap_has_free_bit(&self)
        requires
            self.inv(),
            self@.can_allocate(),
        ensures
            self.index@.has_free_bit(),
    {
        // Strategy: We know |allocated_blocks| < num_data_blocks.
        let num_data: int = self.num_data_blocks as int;
        let num_idx: int = self.num_index_blocks as int;

        assert(self@.allocated_blocks.len() < num_data);

        let full_range: Set<int> = set_int_range(0, num_data);
        lemma_int_range(0, num_data);
        assert(full_range.finite());
        assert(full_range.len() == num_data);

        // Use helper lemmas to prove finiteness and subset properties.
        self.lemma_allocated_blocks_finite();
        assert(self@.allocated_blocks.finite());

        self.lemma_allocated_blocks_subset_of_range();
        assert(self@.allocated_blocks.subset_of(full_range));

        // Prove by contradiction: if all elements of full_range were in allocated_blocks,
        if forall|j: int| #![auto] full_range.contains(j) ==> self@.allocated_blocks.contains(j) {
            assert(full_range.subset_of(self@.allocated_blocks));
            lemma_len_subset(full_range, self@.allocated_blocks);
            assert(full_range.len() <= self@.allocated_blocks.len());
            // Contradiction: num_data <= allocated_blocks.len() < num_data is impossible.
            assert(false);
        }
        let j: int = choose|j: int|
            #![auto]
            full_range.contains(j) && !self@.allocated_blocks.contains(j);

        assert(0 <= j && j < num_data);

        assert(!self@.is_allocated(j));

        let bitmap_idx: int = num_idx + j;

        assert(0 <= bitmap_idx);
        assert(bitmap_idx < self.index@.num_bits);

        assert(!self.index@.is_bit_set(bitmap_idx));

        self.index@.lemma_unset_bit_implies_has_free_bit(bitmap_idx);
        assert(self.index@.has_free_bit());
    }

    //==============================================================================================
    /// Lemma: Block memory regions are disjoint for different block indices.
    /// This proves the no_memory_aliasing property.
    proof fn lemma_blocks_disjoint(view: &SlabView, i: int, j: int)
        requires
            view.block_size > 0,
            0 <= i < view.num_data_blocks,
            0 <= j < view.num_data_blocks,
            i != j,
        ensures
            view.blocks_are_disjoint(i, j),
    {
        // Proof:
        let addr_i = view.block_addr(i);
        let addr_j = view.block_addr(j);
        // Proof:
        let bs = view.block_size;

        assert(addr_i == view.data_addr + i * bs);
        assert(addr_j == view.data_addr + j * bs);

        vstd::arithmetic::mul::lemma_mul_is_distributive_sub(bs, j, i);
        assert(bs * (j - i) == bs * j - bs * i);

        vstd::arithmetic::mul::lemma_mul_is_commutative(bs, j - i);
        vstd::arithmetic::mul::lemma_mul_is_commutative(bs, j);
        vstd::arithmetic::mul::lemma_mul_is_commutative(bs, i);
        assert((j - i) * bs == j * bs - i * bs);

        vstd::arithmetic::mul::lemma_mul_is_distributive_sub(bs, i, j);
        vstd::arithmetic::mul::lemma_mul_is_commutative(bs, i - j);
        assert((i - j) * bs == i * bs - j * bs);

        if i < j {
            let diff = j - i;
            assert(diff >= 1);
            vstd::arithmetic::mul::lemma_mul_inequality(1, diff, bs);
            assert(1 * bs <= diff * bs);
            assert(bs <= diff * bs);
            assert(addr_j - addr_i == j * bs - i * bs);
            assert(addr_j - addr_i == diff * bs);
            assert(addr_j - addr_i >= bs);
            assert(addr_i + bs <= addr_j);
        } else {
            let diff = i - j;
            assert(diff >= 1);
            vstd::arithmetic::mul::lemma_mul_inequality(1, diff, bs);
            assert(1 * bs <= diff * bs);
            assert(bs <= diff * bs);
            assert(addr_i - addr_j == i * bs - j * bs);
            assert(addr_i - addr_j == diff * bs);
            assert(addr_i - addr_j >= bs);
            assert(addr_j + bs <= addr_i);
        }
    }

    /// Lemma: block_addr(addr_to_block_idx(a)) == a for valid addresses.
    /// This proves the inverse relationship for valid addresses.
    proof fn lemma_block_addr_inverse(view: &SlabView, addr: int)
        requires
            view.block_size > 0,
            view.is_valid_addr(addr),
        ensures
            view.block_addr_inverse(addr),
    {
        let bs = view.block_size;
        let data_addr = view.data_addr;
        let offset = addr - data_addr;

        assert(offset >= 0);
        assert(offset % bs == 0);

        assert((offset / bs) * bs == offset) by (nonlinear_arith)
            requires
                bs > 0,
                offset >= 0,
                offset % bs == 0,
        ;

        let k = offset / bs;
        assert(view.addr_to_block_idx(addr) == k);

        assert(view.block_addr(k) == data_addr + k * bs);
        assert(k * bs == offset);
        assert(data_addr + offset == addr);
    }

    //==============================================================================================
    /// Lemma: a * b is always divisible by b (when b > 0).
    proof fn lemma_mul_divisible(a: int, b: int)
        requires
            b > 0,
        ensures
            (a * b) % b == 0,
    {
        assert((a * b) % b == 0) by (nonlinear_arith)
            requires
                b > 0,
        ;
    }

    /// Lemma: (a * b) / b == a (when b > 0).
    proof fn lemma_div_cancel(a: int, b: int)
        requires
            b > 0,
        ensures
            (a * b) / b == a,
    {
        assert((a * b) / b == a) by (nonlinear_arith)
            requires
                b > 0,
        ;
    }

    /// Lemma: if a < b and c > 0, then a * c < b * c.
    proof fn lemma_mul_inequality(a: int, b: int, c: int)
        requires
            a < b,
            c > 0,
        ensures
            a * c < b * c,
    {
        assert(a * c < b * c) by (nonlinear_arith)
            requires
                a < b,
                c > 0,
        ;
    }

    /// Lemma: if q = a / b (integer division), then q * b <= a.
    proof fn lemma_div_mul_le(a: int, b: int)
        requires
            b > 0,
            a >= 0,
        ensures
            (a / b) * b <= a,
    {
        assert((a / b) * b <= a) by (nonlinear_arith)
            requires
                b > 0,
                a >= 0,
        ;
    }

    /// Lemma: distributive property (a + b) * c == a * c + b * c.
    proof fn lemma_distributive(a: int, b: int, c: int)
        ensures
            (a + b) * c == a * c + b * c,
    {
        assert((a + b) * c == a * c + b * c) by (nonlinear_arith);
    }

    /// Trusted bridge: bitwise check `n & (n - 1) == 0` implies `is_pow2(n)`.
    ///
    /// # Trust Justification
    ///
    /// n & (n - 1) == 0 iff n is a power of two. See Hacker's Delight, Chapter 2.
    #[verifier::external_body]
    proof fn lemma_bitwise_implies_is_pow2(n: usize)
        requires
            n > 0,
            n & sub(n, 1) == 0,
        ensures
            is_pow2(n as int),
    {
    }

    /// Proves that after a failed clear, slab invariant is preserved.
    proof fn lemma_dealloc_clear_err_preserves_inv(slab: &Slab, old_slab: &Slab)
        requires
            old_slab.inv(),
            slab.index.inv(),
            slab.index@.set_bits =~= old_slab.index@.set_bits,
            slab.index@.num_bits == old_slab.index@.num_bits,
            slab.num_index_blocks == old_slab.num_index_blocks,
            slab.num_data_blocks == old_slab.num_data_blocks,
            slab.block_size == old_slab.block_size,
            slab.data_addr == old_slab.data_addr,
        ensures
            slab.inv(),
    {
        // Prove all index blocks are still set (using set_bits trigger for lemma_inv_from_components).
        assert forall|j: int| 0 <= j < slab.num_index_blocks as int implies slab.index@.is_bit_set(
            j,
        ) by {
            assert(old_slab.index@.is_bit_set(j));
        }
        // Prove inv().
        Self::lemma_inv_from_components(slab);
    }

    //==============================================================================================
    // Extracted proof-block lemmas
    //==============================================================================================
    /// Lemma: Proves layout bounds during slab construction.
    /// Establishes that num_index_blocks >= 1, num_data_blocks > 0,
    /// and the product num_index_blocks * block_size is bounded.
    proof fn lemma_from_raw_parts_layout_bounds(
        len: usize,
        block_size: usize,
        total_num_blocks: usize,
        index_len: usize,
        num_index_blocks: usize,
        num_data_blocks: usize,
        addr: usize,
    )
        requires
            len > 0,
            len < i32::MAX as usize,
            block_size > 0,
            block_size <= len,
            addr > 0,
            addr as int + len as int <= usize::MAX as int,
            total_num_blocks == len / block_size,
            total_num_blocks >= 8,
            index_len == total_num_blocks / 8,
            num_index_blocks == index_len / block_size + (if index_len % block_size == 0 {
                0usize
            } else {
                1usize
            }),
            num_index_blocks <= total_num_blocks,
            num_data_blocks == total_num_blocks - num_index_blocks,
        ensures
            num_index_blocks >= 1,
            num_data_blocks > 0,
            (num_index_blocks as int) * (block_size as int) < (total_num_blocks as int) * (
            block_size as int),
            (total_num_blocks as int) * (block_size as int) <= len as int,
            (addr as int) + (num_index_blocks as int) * (block_size as int) <= (addr as int) + (
            len as int),
            (addr as int) + (num_index_blocks as int) * (block_size as int) <= usize::MAX as int,
    {
        assert(index_len >= 1);
        // Prove num_index_blocks >= 1.
        if index_len >= block_size {
            assert(index_len / block_size >= 1) by (nonlinear_arith)
                requires
                    index_len >= block_size,
                    block_size > 0int,
            ;
        } else {
            assert(index_len % block_size == index_len) by (nonlinear_arith)
                requires
                    0 < index_len < block_size,
            ;
            assert(index_len % block_size != 0);
        }
        assert(num_index_blocks >= 1);
        // Prove num_data_blocks > 0.
        assert(num_index_blocks <= index_len + 1) by {
            assert(index_len / block_size <= index_len) by (nonlinear_arith)
                requires
                    block_size >= 1int,
                    index_len >= 0int,
            ;
        }
        assert(index_len + 1 <= total_num_blocks) by {
            assert(total_num_blocks / 8 + 1 <= total_num_blocks) by (nonlinear_arith)
                requires
                    total_num_blocks >= 8int,
            ;
        }
        assert(num_index_blocks < total_num_blocks);
        assert(num_data_blocks > 0);
        // Prove product bounds.
        Self::lemma_div_mul_le(len as int, block_size as int);
        Self::lemma_mul_inequality(
            num_index_blocks as int,
            total_num_blocks as int,
            block_size as int,
        );
    }

    /// Lemma: Converts is_zero properties on a byte sequence to concrete equality with 0u8.
    proof fn lemma_raw_array_storage_zeroed(s: Seq<u8>)
        requires
            forall|i: int| 0 <= i < s.len() ==> is_zero(#[trigger] s[i]),
        ensures
            forall|i: int| 0 <= i < s.len() ==> s[i] == 0u8,
    {
        assert forall|i: int| 0 <= i < s.len() implies s[i] == 0u8 by {
            axiom_u8_zero_is_0(s[i]);
        }
    }

    /// Lemma: Establishes pre-loop invariants for from_raw_parts.
    /// Connects bitmap num_bits to total_num_blocks and layout.
    proof fn lemma_from_raw_parts_pre_loop(
        index_nbits: int,
        index_len: int,
        total_num_blocks: int,
        num_index_blocks: int,
        num_data_blocks: int,
    )
        requires
            index_nbits == index_len * 8,
            total_num_blocks == index_len * 8,
            num_index_blocks + num_data_blocks == total_num_blocks,
            num_index_blocks < total_num_blocks,
        ensures
            index_nbits == total_num_blocks,
            num_index_blocks + num_data_blocks == index_nbits,
    {
    }

    /// Lemma: Proves postconditions after the initialization loop in from_raw_parts.
    /// Establishes slab invariant, view fields, and emptiness.
    proof fn lemma_from_raw_parts_post_loop(slab: &Slab, addr: int, len: int, total_num_blocks: int)
        requires
            slab.index.inv(),
            slab.block_size > 0,
            slab.num_data_blocks > 0,
            slab.num_index_blocks > 0,
            forall|i: int|
                #![trigger slab.index@.set_bits.contains(i)]
                0 <= i < slab.num_index_blocks as int ==> slab.index@.set_bits.contains(i),
            forall|i: int|
                slab.num_index_blocks as int <= i < slab.index@.num_bits
                    ==> !slab.index@.is_bit_set(i),
            (slab.num_index_blocks as int) + (slab.num_data_blocks as int) == slab.index@.num_bits,
            total_num_blocks == slab.index@.num_bits,
            (slab.num_data_blocks as int) < total_num_blocks,
            (slab.data_addr as int) == addr + (slab.num_index_blocks as int) * (
            slab.block_size as int),
            addr > 0,
            len > 0,
            len < i32::MAX as int,
            slab.block_size as int <= len,
            addr + len <= usize::MAX as int,
            total_num_blocks == len / slab.block_size as int,
            is_pow2(slab.block_size as int),
            slab.data_addr as int % slab.block_size as int == 0,
        ensures
            slab.inv(),
            slab@.block_size == slab.block_size as int,
            slab@.allocated_blocks =~= Set::<int>::empty(),
            slab@.data_addr > addr,
            slab@.data_addr % slab.block_size as int == 0,
            slab@.num_data_blocks > 0,
            slab@.data_addr + slab@.num_data_blocks * slab@.block_size <= addr + len,
    {
        // Prove memory bounds.
        Self::lemma_div_mul_le(len, slab.block_size as int);
        assert(total_num_blocks == len / slab.block_size as int);
        assert(total_num_blocks * slab.block_size as int <= len);
        Self::lemma_mul_inequality(
            slab.num_data_blocks as int,
            total_num_blocks,
            slab.block_size as int,
        );
        assert((slab.num_data_blocks as int) * (slab.block_size as int) < total_num_blocks
            * slab.block_size as int);
        assert(len < usize::MAX as int);
        assert((slab.num_data_blocks as int) * (slab.block_size as int) <= usize::MAX as int);

        // Prove metadata/data disjointness.
        assert(slab.data_addr as int == addr + slab.num_index_blocks as int
            * slab.block_size as int);
        Self::lemma_distributive(
            slab.num_index_blocks as int,
            slab.num_data_blocks as int,
            slab.block_size as int,
        );
        assert(slab.num_index_blocks as int * slab.block_size as int + slab.num_data_blocks as int
            * slab.block_size as int == (slab.num_index_blocks as int + slab.num_data_blocks as int)
            * slab.block_size as int);
        assert(slab.data_addr as int + slab.num_data_blocks as int * slab.block_size as int == addr
            + (slab.num_index_blocks as int + slab.num_data_blocks as int)
            * slab.block_size as int);
        assert(slab.num_index_blocks as int + slab.num_data_blocks as int == total_num_blocks);
        assert(slab.data_addr as int + slab.num_data_blocks as int * slab.block_size as int == addr
            + total_num_blocks * slab.block_size as int);
        assert(total_num_blocks * slab.block_size as int <= len);
        assert(addr + total_num_blocks * slab.block_size as int <= addr + len);
        assert(addr + len <= usize::MAX as int);
        assert(slab.data_addr as int + slab.num_data_blocks as int * slab.block_size as int
            <= usize::MAX as int);

        assert(slab.data_addr as int == addr + slab.num_index_blocks as int
            * slab.block_size as int);
        assert(addr > 0);
        assert(slab.data_addr as int > slab.num_index_blocks as int * slab.block_size as int);
        assert(slab.data_addr as int >= slab.num_index_blocks as int * slab.block_size as int);

        Self::lemma_inv_from_components(slab);
        Self::lemma_view_fields(slab);
        Self::lemma_new_slab_is_empty(slab);
        assert(slab@.allocated_blocks =~= Set::<int>::empty()) by {
            assert forall|i: int| !slab@.allocated_blocks.contains(i) by {
                if 0 <= i < slab@.num_data_blocks {
                    assert(!slab@.is_allocated(i));
                } else {
                    assert(slab@.allocated_blocks_in_range());
                    assert(!slab@.is_allocated(i));
                }
            }
        }
        assert(slab.data_addr as int > addr);
        assert(slab.data_addr as int % slab.block_size as int == 0);
        assert(slab@.data_addr + slab@.num_data_blocks * slab@.block_size <= addr + len);
    }

    /// Lemma: When bitmap alloc fails, slab state is preserved and cannot allocate.
    proof fn lemma_alloc_error_preserves_state(slab: &Slab, old_slab: &Slab)
        requires
            old_slab.inv(),
            slab.index.inv(),
            slab.index@.set_bits =~= old_slab.index@.set_bits,
            slab.index@.num_bits == old_slab.index@.num_bits,
            slab.num_index_blocks == old_slab.num_index_blocks,
            slab.num_data_blocks == old_slab.num_data_blocks,
            slab.block_size == old_slab.block_size,
            slab.data_addr == old_slab.data_addr,
            !slab.index@.has_free_bit(),
        ensures
            slab.inv(),
            slab@ == old_slab@,
            !old_slab@.can_allocate(),
    {
        // Liveness: if can_allocate(), bitmap has_free_bit - contradiction.
        if old_slab@.can_allocate() {
            old_slab.lemma_can_allocate_implies_bitmap_has_free_bit();
            assert(false);
        }
        // Prove index blocks still set.

        assert forall|i: int|
            0 <= i < slab.num_index_blocks as int implies #[trigger] slab.index@.set_bits.contains(
            i,
        ) by {
            assert(old_slab.index@.set_bits.contains(i));
            assert(slab.index@.set_bits.contains(i) == old_slab.index@.set_bits.contains(i));
        }
        Self::lemma_inv_from_components(slab);
        // Prove view equality.
        assert(slab@.num_data_blocks == old_slab@.num_data_blocks);
        assert(slab@.block_size == old_slab@.block_size);
        assert(slab@.data_addr == old_slab@.data_addr);
        assert(slab@.allocated_blocks =~= old_slab@.allocated_blocks) by {
            assert forall|j: int| 0 <= j < slab.num_data_blocks as int implies (
            slab@.allocated_blocks.contains(j) == old_slab@.allocated_blocks.contains(j)) by {
                let bitmap_idx: int = slab.num_index_blocks as int + j;
                assert(slab.index@.set_bits.contains(bitmap_idx)
                    == old_slab.index@.set_bits.contains(bitmap_idx));
            }
        }
        assert(slab@ == old_slab@);
    }

    /// Lemma: After bitmap alloc, the returned bit is a data block with valid bounds.
    proof fn lemma_alloc_block_is_data_block_with_bounds(slab: &Slab, old_slab: &Slab, block: int)
        requires
            old_slab.inv(),
            slab.inv(),
            0 <= block < old_slab.index@.num_bits,
            !old_slab.index@.is_bit_set(block),
            slab.index@.is_bit_set(block),
            slab.num_index_blocks == old_slab.num_index_blocks,
            slab.num_data_blocks == old_slab.num_data_blocks,
            slab.block_size == old_slab.block_size,
            slab.data_addr == old_slab.data_addr,
            slab.index@.num_bits == old_slab.index@.num_bits,
        ensures
            block >= slab.num_index_blocks as int,
            block < (slab.num_index_blocks + slab.num_data_blocks) as int,
    {
        assert(block >= slab.num_index_blocks as int) by {
            if block < slab.num_index_blocks as int {
                assert(old_slab.index@.is_bit_set(block));
            }
        };
        assert(old_slab.num_index_blocks as int + old_slab.num_data_blocks as int
            == slab.index@.num_bits);
        assert(block < slab.index@.num_bits);
        assert(block < (slab.num_index_blocks + slab.num_data_blocks) as int);
    }

    /// Lemma: block_idx * block_size and data_addr + block_idx * block_size fit in usize.
    proof fn lemma_alloc_product_in_bounds(slab: &Slab, block_idx: int)
        requires
            slab.inv(),
            0 <= block_idx < (slab.num_data_blocks as int),
        ensures
            block_idx * (slab.block_size as int) < (slab.num_data_blocks as int) * (
            slab.block_size as int),
            block_idx * (slab.block_size as int) <= usize::MAX as int,
            (slab.data_addr as int) + block_idx * (slab.block_size as int) <= usize::MAX as int,
    {
        Self::lemma_mul_inequality(block_idx, slab.num_data_blocks as int, slab.block_size as int);
    }

    /// Lemma: Establishes allocate postconditions (address validity, block index, frame).
    proof fn lemma_alloc_establishes_postconditions(
        slab: &Slab,
        old_slab: &Slab,
        block: int,
        block_idx: int,
        block_addr: int,
    )
        requires
            old_slab.inv(),
            slab.inv(),
            block_idx == block - (slab.num_index_blocks as int),
            0 <= block_idx < (slab.num_data_blocks as int),
            block_addr == (slab.data_addr as int) + block_idx * (slab.block_size as int),
            slab.index@.is_bit_set((slab.num_index_blocks as int) + block_idx),
            !old_slab.index@.is_bit_set((slab.num_index_blocks as int) + block_idx),
            forall|k: int|
                k != block && 0 <= k < slab.index@.num_bits ==> slab.index@.is_bit_set(k)
                    == old_slab.index@.is_bit_set(k),
            slab.num_index_blocks == old_slab.num_index_blocks,
            slab.num_data_blocks == old_slab.num_data_blocks,
            slab.block_size == old_slab.block_size,
            slab.data_addr == old_slab.data_addr,
        ensures
            old_slab@.is_valid_addr(block_addr),
            old_slab@.addr_to_block_idx(block_addr) == block_idx,
            !old_slab@.is_allocated(block_idx),
            slab@.is_allocated(block_idx),
            slab@.num_data_blocks == old_slab@.num_data_blocks,
            slab@.block_size == old_slab@.block_size,
            slab@.data_addr == old_slab@.data_addr,
            slab@.allocated_blocks =~= old_slab@.allocated_blocks.insert(block_idx),
            block_addr > 0,
    {
        Self::lemma_view_fields(old_slab);
        let bs: int = slab.block_size as int;
        let ndb: int = slab.num_data_blocks as int;

        // Address validity proofs.
        assert(block_addr == slab.data_addr as int + block_idx * bs);
        Self::lemma_mul_inequality(block_idx, ndb, bs);
        assert(block_addr < slab.data_addr as int + ndb * bs);
        Self::lemma_mul_divisible(block_idx, bs);
        assert((block_addr - slab.data_addr as int) % bs == 0);
        assert(old_slab@.is_valid_addr(block_addr));

        // Block index computation.
        Self::lemma_div_cancel(block_idx, bs);
        assert(old_slab@.addr_to_block_idx(block_addr) == block_idx);

        // Allocation status.
        assert(!old_slab@.is_allocated(block_idx));
        assert(slab@.is_allocated(block_idx));

        // Other bits unchanged.
        assert forall|i: int|
            0 <= i < ndb && i != block_idx implies #[trigger] slab.index@.is_bit_set(
            slab.num_index_blocks as int + i,
        ) == #[trigger] old_slab.index@.is_bit_set(slab.num_index_blocks as int + i) by {
            let global_idx: int = slab.num_index_blocks as int + i;
            if global_idx == block {
                assert(i == block_idx);
            }
        }
        // Frame for allocated_blocks.
        assert forall|i: int| 0 <= i < ndb && i != block_idx implies slab@.is_allocated(i)
            == old_slab@.is_allocated(i) by {
            let bitmap_idx: int = slab.num_index_blocks as int + i;
            assert(slab.index@.is_bit_set(bitmap_idx) == old_slab.index@.is_bit_set(bitmap_idx));
        }
        // Prove allocated_blocks =~= old_allocated_blocks.insert(block_idx).
        assert(slab@.allocated_blocks =~= old_slab@.allocated_blocks.insert(block_idx)) by {
            assert forall|j: int|
                #![auto]
                slab@.allocated_blocks.contains(j) == old_slab@.allocated_blocks.insert(
                    block_idx,
                ).contains(j) by {
                if j == block_idx {
                    assert(slab@.is_allocated(block_idx));
                } else if 0 <= j < ndb {
                    assert(slab@.is_allocated(j) == old_slab@.is_allocated(j));
                } else {
                    assert(slab@.allocated_blocks_in_range());
                    assert(!slab@.is_allocated(j));
                    assert(old_slab@.allocated_blocks_in_range());
                    assert(!old_slab@.is_allocated(j));
                }
            }
        }
    }

    /// Lemma: Proves offset and index bounds for deallocate.
    proof fn lemma_dealloc_offset_bounds(slab: &Slab, ptr: int)
        requires
            slab.inv(),
            slab@.is_valid_addr(ptr),
            slab@.can_deallocate(slab@.addr_to_block_idx(ptr)),
        ensures
            ptr >= (slab.data_addr as int),
            (ptr - (slab.data_addr as int)) >= 0,
            (ptr - (slab.data_addr as int)) < (slab.num_data_blocks as int) * (
            slab.block_size as int),
            ((ptr - (slab.data_addr as int)) % (slab.block_size as int)) == 0,
            ({
                let block_idx: int = (ptr - (slab.data_addr as int)) / (slab.block_size as int);
                &&& 0 <= block_idx < (slab.num_data_blocks as int)
                &&& (slab.num_index_blocks as int) + block_idx < slab.index@.num_bits
                &&& (slab.num_index_blocks as int) + block_idx < usize::MAX as int
            }),
    {
        assert(ptr >= slab.data_addr as int);
        assert(ptr < slab.data_addr as int + slab.num_data_blocks as int * slab.block_size as int);
        assert((ptr - slab.data_addr as int) % slab.block_size as int == 0);

        let offset: int = ptr - slab.data_addr as int;
        assert(offset >= 0);
        assert(offset < slab.num_data_blocks as int * slab.block_size as int);

        let block_idx: int = offset / slab.block_size as int;
        assert(0 <= block_idx < slab.num_data_blocks as int);
        assert(slab.num_index_blocks as int + block_idx < slab.index@.num_bits);

        // Prove no overflow for usize computation.
        assert(slab.num_index_blocks as int + block_idx < slab.num_index_blocks as int
            + slab.num_data_blocks as int);
        assert(slab.num_index_blocks as int + slab.num_data_blocks as int == slab.index@.num_bits);
        Self::lemma_slab_inv_implies_bitmap_inv(slab);
        assert(slab.index@.num_bits <= usize::MAX as int);
        assert(slab.num_index_blocks as int + block_idx < usize::MAX as int);
    }

    /// Lemma: Connects the computed index to addr_to_block_idx and proves the bit is set.
    proof fn lemma_dealloc_index_is_allocated(slab: &Slab, ptr: int, index: int)
        requires
            slab.inv(),
            slab@.is_valid_addr(ptr),
            slab@.can_deallocate(slab@.addr_to_block_idx(ptr)),
            index == (slab.num_index_blocks as int) + (ptr - (slab.data_addr as int)) / (
            slab.block_size as int),
            index < slab.index@.num_bits,
        ensures
            slab.index@.is_bit_set(index),
            slab@.is_allocated(slab@.addr_to_block_idx(ptr)),
            index == (slab.num_index_blocks as int) + slab@.addr_to_block_idx(ptr),
    {
        let block_idx_spec: int = slab@.addr_to_block_idx(ptr);
        assert(block_idx_spec == (ptr - slab.data_addr as int) / slab.block_size as int);
        assert(index == slab.num_index_blocks as int + block_idx_spec);
        assert(slab@.is_allocated(block_idx_spec));
        assert(slab.index@.is_bit_set(index));
    }

    /// Lemma: After a successful clear in deallocate, proves inv, can_allocate, and frame.
    proof fn lemma_dealloc_clear_ok_postconditions(
        slab: &Slab,
        old_slab: &Slab,
        index: int,
        ptr: int,
    )
        requires
            old_slab.inv(),
            slab.index.inv(),
            slab.index@.num_bits == old_slab.index@.num_bits,
            slab.num_index_blocks == old_slab.num_index_blocks,
            slab.num_data_blocks == old_slab.num_data_blocks,
            slab.block_size == old_slab.block_size,
            slab.data_addr == old_slab.data_addr,
            !slab.index@.is_bit_set(index),
            old_slab.index@.is_bit_set(index),
            forall|j: int|
                j != index && 0 <= j < slab.index@.num_bits ==> slab.index@.is_bit_set(j)
                    == old_slab.index@.is_bit_set(j),
            index == (old_slab.num_index_blocks as int) + old_slab@.addr_to_block_idx(ptr),
            old_slab@.is_valid_addr(ptr),
            old_slab@.can_deallocate(old_slab@.addr_to_block_idx(ptr)),
        ensures
            slab.inv(),
            ({
                let block_idx_spec: int = old_slab@.addr_to_block_idx(ptr);
                &&& !slab@.is_allocated(block_idx_spec)
                &&& slab@.num_data_blocks == old_slab@.num_data_blocks
                &&& slab@.block_size == old_slab@.block_size
                &&& slab@.data_addr == old_slab@.data_addr
                &&& slab@.allocated_blocks =~= old_slab@.allocated_blocks.remove(block_idx_spec)
                &&& slab@.can_allocate()
            }),
    {
        let block_idx_spec: int = old_slab@.addr_to_block_idx(ptr);

        assert forall|j: int|
            0 <= j < slab.num_index_blocks as int implies #[trigger] slab.index@.set_bits.contains(
            j,
        ) by {
            assert(old_slab.index@.is_bit_set(j));
        }
        Self::lemma_inv_from_components(slab);

        slab.index@.lemma_unset_bit_implies_has_free_bit(index);

        // Prove allocated_blocks.len() < num_data_blocks.
        slab.lemma_allocated_blocks_finite();
        slab.lemma_allocated_blocks_subset_of_range();
        let full_range: Set<int> = set_int_range(0, slab@.num_data_blocks);
        lemma_int_range(0, slab@.num_data_blocks);

        lemma_len_subset(slab@.allocated_blocks, full_range);

        if slab@.allocated_blocks =~= full_range {
        }
        assert(slab@.allocated_blocks.len() < slab@.num_data_blocks) by {
            let with_witness: Set<int> = slab@.allocated_blocks.insert(block_idx_spec);
            assert forall|x: int| with_witness.contains(x) implies full_range.contains(x) by {
                if x == block_idx_spec {
                } else {
                }
            }
            axiom_set_insert_len(slab@.allocated_blocks, block_idx_spec);
            lemma_len_subset(with_witness, full_range);
        }

        // Prove allocated_blocks =~= old_allocated_blocks.remove(block_idx_spec).
        assert(slab@.allocated_blocks =~= old_slab@.allocated_blocks.remove(block_idx_spec)) by {
            assert forall|j: int|
                #![auto]
                slab@.allocated_blocks.contains(j) == old_slab@.allocated_blocks.remove(
                    block_idx_spec,
                ).contains(j) by {
                if j == block_idx_spec {
                } else if 0 <= j < slab@.num_data_blocks {
                    let bitmap_idx: int = slab.num_index_blocks as int + j;
                    assert(slab.index@.is_bit_set(bitmap_idx) == old_slab.index@.is_bit_set(
                        bitmap_idx,
                    ));
                } else {
                }
            }
        }
    }

    /// Combined lemma: dealloc clear postconditions + permission well-formedness.
    /// Wraps lemma_dealloc_clear_ok_postconditions, lemma_block_addr_inverse,
    /// and lemma_dealloc_perms_wf into a single call for proof block extraction.
    proof fn lemma_dealloc_clear_ok_with_perms(
        slab: &Slab,
        old_slab: &Slab,
        index: int,
        ptr: int,
        old_free_perms: Map<int, PointsToRaw>,
        prov: Provenance,
    )
        requires
            old_slab.inv(),
            slab.index.inv(),
            slab.index@.num_bits == old_slab.index@.num_bits,
            slab.num_index_blocks == old_slab.num_index_blocks,
            slab.num_data_blocks == old_slab.num_data_blocks,
            slab.block_size == old_slab.block_size,
            slab.data_addr == old_slab.data_addr,
            !slab.index@.is_bit_set(index),
            old_slab.index@.is_bit_set(index),
            forall|j: int|
                j != index && 0 <= j < slab.index@.num_bits ==> slab.index@.is_bit_set(j)
                    == old_slab.index@.is_bit_set(j),
            index == (old_slab.num_index_blocks as int) + old_slab@.addr_to_block_idx(ptr),
            old_slab@.is_valid_addr(ptr),
            old_slab@.can_deallocate(old_slab@.addr_to_block_idx(ptr)),
            old_slab@.perms_wf(old_free_perms, prov),
        ensures
            slab.inv(),
            ({
                let block_idx_spec: int = old_slab@.addr_to_block_idx(ptr);
                &&& !slab@.is_allocated(block_idx_spec)
                &&& slab@.num_data_blocks == old_slab@.num_data_blocks
                &&& slab@.block_size == old_slab@.block_size
                &&& slab@.data_addr == old_slab@.data_addr
                &&& slab@.allocated_blocks =~= old_slab@.allocated_blocks.remove(block_idx_spec)
                &&& slab@.can_allocate()
            }),
    {
        Self::lemma_dealloc_clear_ok_postconditions(slab, old_slab, index, ptr);
        Self::lemma_block_addr_inverse(&old_slab@, ptr);
    }

    //==================================================================================================
    // Verified Safety Properties (not called by exec code, but prove key allocator properties)
    //==================================================================================================
    /// Lemma: addr_to_block_idx(block_addr(i)) == i for valid block index i.
    /// This proves the inverse relationship between address and block index.
    proof fn lemma_addr_block_idx_inverse(view: &SlabView, i: int)
        requires
            view.block_size > 0,
            0 <= i < view.num_data_blocks,
        ensures
            view.addr_block_idx_inverse(i),
    {
        let bs = view.block_size;
        let addr = view.block_addr(i);

        vstd::arithmetic::div_mod::lemma_div_by_multiple(i, bs);

    }

    /// Lemma: After allocating block_idx from a well-formed permission map,
    /// removing the block's permission yields a map that is well-formed for the new slab state.
    proof fn lemma_alloc_perms_wf(
        old_view: SlabView,
        new_view: SlabView,
        perms: Map<int, PointsToRaw>,
        block_idx: int,
        prov: Provenance,
    )
        requires
            old_view.block_size > 0,
            old_view.num_data_blocks > 0,
            old_view.perms_wf(perms, prov),
            0 <= block_idx < old_view.num_data_blocks,
            !old_view.is_allocated(block_idx),
            new_view.num_data_blocks == old_view.num_data_blocks,
            new_view.block_size == old_view.block_size,
            new_view.data_addr == old_view.data_addr,
            new_view.allocated_blocks =~= old_view.allocated_blocks.insert(block_idx),
        ensures
            perms.dom().contains(block_idx),
            perms[block_idx].is_range(old_view.block_addr(block_idx), old_view.block_size),
            perms[block_idx].provenance() == prov,
            new_view.perms_wf(perms.remove(block_idx), prov),
    {
    }

    /// Lemma: All allocated blocks are within valid range.
    proof fn lemma_allocated_blocks_in_range(&self)
        requires
            self.inv(),
        ensures
            self@.allocated_blocks_in_range(),
    {
    }

    /// Lemma (Liveness): If bitmap is full, then slab is full.
    /// This is the converse of lemma_can_allocate_implies_bitmap_has_free_bit.
    /// It connects bitmap fullness to slab fullness.
    proof fn lemma_bitmap_full_implies_slab_full(&self)
        requires
            self.inv(),
            self.index@.is_full(),
        ensures
            self@.is_full(),
    {
        let num_data: int = self.num_data_blocks as int;
        let num_idx: int = self.num_index_blocks as int;

        // Prove all data block indices are allocated.
        assert forall|j: int| 0 <= j < num_data implies self@.is_allocated(j) by {
            let bitmap_idx = num_idx + j;
            assert(self.index@.is_bit_set(bitmap_idx));  // From bitmap.is_full() via lemma
        }

        let full_range: Set<int> = set_int_range(0, num_data);
        lemma_int_range(0, num_data);

        assert forall|j: int|
            #![auto]
            full_range.contains(j) implies self@.allocated_blocks.contains(j) by {}

        assert forall|j: int|
            #![auto]
            self@.allocated_blocks.contains(j) implies full_range.contains(j) by {}

        assert(self@.allocated_blocks =~= full_range);

        self.lemma_allocated_blocks_finite();
    }

    /// Lemma (Liveness): Deallocation from full slab enables allocation.
    /// If the slab was full, after deallocating one block, allocation becomes possible.
    proof fn lemma_dealloc_from_full_enables_alloc(&self, new_self: &Self, block_idx: int)
        requires
            self.inv(),
            new_self.inv(),
            self@.used() == self@.capacity(),  // Slab was full
            0 <= block_idx < self@.num_data_blocks,
            self@.is_allocated(block_idx),
            !new_self@.is_allocated(block_idx),
            forall|i: int|
                (0 <= i < self@.num_data_blocks && i != block_idx) ==> (self@.is_allocated(i)
                    <==> new_self@.is_allocated(i)),
            new_self@.num_data_blocks == self@.num_data_blocks,
        ensures
            new_self@.can_allocate(),
            new_self@.free() >= 1,
    {
        // Step 3: new_self@.allocated_blocks is a subset of self@.allocated_blocks.remove(block_idx)
        let old_set: Set<int> = self@.allocated_blocks;
        let new_set: Set<int> = new_self@.allocated_blocks;
        let removed_set: Set<int> = old_set.remove(block_idx);

        // Prove old_set is finite: it's a subset of set_int_range(0, num_data_blocks)
        let range_set: Set<int> = set_int_range(0, self@.num_data_blocks);

        // Prove old_set is a subset of range_set
        assert forall|i: int| old_set.contains(i) implies range_set.contains(i) by {}

        lemma_int_range(0, self@.num_data_blocks);

        lemma_set_subset_finite(range_set, old_set);

        // Similarly prove new_set is finite
        let new_range_set: Set<int> = set_int_range(0, new_self@.num_data_blocks);
        assert forall|i: int| new_set.contains(i) implies new_range_set.contains(i) by {}
        lemma_int_range(0, new_self@.num_data_blocks);
        lemma_set_subset_finite(new_range_set, new_set);

        assert forall|i: int| new_set.contains(i) implies removed_set.contains(i) by {
            if new_set.contains(i) {
                assert(new_self@.is_allocated(i));
            }
        }

        axiom_set_remove_len(old_set, block_idx);

        lemma_len_subset(new_set, removed_set);

    }

    /// Lemma: A freshly initialized slab has maximum free capacity.
    proof fn lemma_fresh_slab_max_free(slab: &Slab)
        requires
            slab.inv(),
            slab@.is_freshly_initialized(),
        ensures
            slab@.free() == slab@.capacity(),
            slab@.used() == 0,
    {
    }

    /// Lemma: A newly created slab is freshly initialized (no data blocks allocated).
    proof fn lemma_new_slab_freshly_initialized(slab: &Slab)
        requires
            slab.inv(),
            forall|i: int|
                slab.num_index_blocks as int <= i < (slab.num_index_blocks
                    + slab.num_data_blocks) as int ==> !slab.index@.is_bit_set(i),
        ensures
            slab@.is_freshly_initialized(),
    {
        assert(slab@.allocated_blocks =~= Set::<int>::empty()) by {
            assert forall|i: int| !slab@.allocated_blocks.contains(i) by {
                if 0 <= i < slab@.num_data_blocks {
                    let bitmap_idx = slab.num_index_blocks as int + i;
                    assert(!slab.index@.is_bit_set(bitmap_idx));
                }
            }
        }
    }

    /// Lemma: No memory aliasing - all allocated blocks have disjoint regions.
    proof fn lemma_no_memory_aliasing(&self)
        requires
            self.inv(),
        ensures
            self@.no_memory_aliasing(),
    {
        // For any two allocated blocks i, j with i != j:
        assert forall|i: int, j: int|
            (self@.is_allocated(i) && self@.is_allocated(j) && i
                != j) implies self@.blocks_are_disjoint(i, j) by {
            if self@.is_allocated(i) && self@.is_allocated(j) && i != j {
                Self::lemma_blocks_disjoint(&self@, i, j);
            }
        }
    }

    //==============================================================================================
    // Extracted proof-block lemmas (proof extraction pass)
    //==============================================================================================
    /// Lemma: Proves set-theoretic properties needed for memory permission splitting
    /// in `from_raw_parts`. Establishes that the used region is a subset of the memory
    /// region, the index region is a subset of the used region, and the data region
    /// equals the difference of the used and index regions.
    proof fn lemma_from_raw_parts_mem_split_properties(
        addr: int,
        len: int,
        block_size: int,
        total_num_blocks: int,
        num_index_blocks: int,
        num_data_blocks: int,
        data_addr: int,
    )
        requires
            len > 0,
            block_size > 0,
            total_num_blocks == len / block_size,
            num_index_blocks + num_data_blocks == total_num_blocks,
            num_index_blocks >= 1,
            num_index_blocks < total_num_blocks,
            num_data_blocks > 0,
            data_addr == addr + num_index_blocks * block_size,
        ensures
            total_num_blocks * block_size <= len,
            (num_index_blocks + num_data_blocks) * block_size == num_index_blocks * block_size
                + num_data_blocks * block_size,
            set_int_range(addr, addr + total_num_blocks * block_size).difference(
                set_int_range(addr, addr + num_index_blocks * block_size),
            ) =~= set_int_range(data_addr, data_addr + num_data_blocks * block_size),
    {
        Self::lemma_div_mul_le(len, block_size);
        Self::lemma_mul_inequality(num_index_blocks, total_num_blocks, block_size);
        Self::lemma_distributive(num_index_blocks, num_data_blocks, block_size);

        assert forall|x: int|
            #![trigger set_int_range(addr, addr + total_num_blocks * block_size).contains(x)]
            set_int_range(addr, addr + total_num_blocks * block_size).contains(
                x,
            ) implies set_int_range(addr, addr + len).contains(x) by {}

        assert forall|x: int|
            #![trigger set_int_range(addr, addr + num_index_blocks * block_size).contains(x)]
            set_int_range(addr, addr + num_index_blocks * block_size).contains(
                x,
            ) implies set_int_range(addr, addr + total_num_blocks * block_size).contains(x) by {}

        assert(set_int_range(addr, addr + total_num_blocks * block_size).difference(
            set_int_range(addr, addr + num_index_blocks * block_size),
        ) =~= set_int_range(data_addr, data_addr + num_data_blocks * block_size)) by {
            let used: Set<int> = set_int_range(addr, addr + total_num_blocks * block_size);
            let idx: Set<int> = set_int_range(addr, addr + num_index_blocks * block_size);
            let data: Set<int> = set_int_range(data_addr, data_addr + num_data_blocks * block_size);
            assert forall|x: int|
                #![trigger used.difference(idx).contains(x)]
                #![trigger data.contains(x)]
                used.difference(idx).contains(x) <==> data.contains(x) by {}
        }
    }

    /// Lemma: Proves slab invariant and permission well-formedness after
    /// `from_raw_parts` initialization. Combines the post-loop invariant proof
    /// with the freshly initialized permission well-formedness proof.
    proof fn lemma_from_raw_parts_finalize(
        slab: &Slab,
        addr: int,
        len: int,
        total_num_blocks: int,
        free_perms: Map<int, PointsToRaw>,
        prov: Provenance,
    )
        requires
            slab.index.inv(),
            slab.block_size > 0,
            slab.num_data_blocks > 0,
            slab.num_index_blocks > 0,
            forall|i: int|
                #![trigger slab.index@.set_bits.contains(i)]
                0 <= i < slab.num_index_blocks as int ==> slab.index@.set_bits.contains(i),
            forall|i: int|
                slab.num_index_blocks as int <= i < slab.index@.num_bits
                    ==> !slab.index@.is_bit_set(i),
            (slab.num_index_blocks as int) + (slab.num_data_blocks as int) == slab.index@.num_bits,
            total_num_blocks == slab.index@.num_bits,
            (slab.num_data_blocks as int) < total_num_blocks,
            (slab.data_addr as int) == addr + (slab.num_index_blocks as int) * (
            slab.block_size as int),
            addr > 0,
            len > 0,
            len < i32::MAX as int,
            slab.block_size as int <= len,
            addr + len <= usize::MAX as int,
            total_num_blocks == len / slab.block_size as int,
            is_pow2(slab.block_size as int),
            slab.data_addr as int % slab.block_size as int == 0,
            forall|i: int| 0 <= i < slab.num_data_blocks as int <==> free_perms.dom().contains(i),
            forall|i: int|
                #![trigger free_perms[i]]
                0 <= i < slab.num_data_blocks as int ==> free_perms[i].is_range(
                    slab.data_addr as int + i * slab.block_size as int,
                    slab.block_size as int,
                ) && free_perms[i].provenance() == prov,
        ensures
            slab.inv(),
            slab@.block_size == slab.block_size as int,
            slab@.allocated_blocks =~= Set::<int>::empty(),
            slab@.data_addr > addr,
            slab@.data_addr % slab.block_size as int == 0,
            slab@.num_data_blocks > 0,
            slab@.data_addr + slab@.num_data_blocks * slab@.block_size <= addr + len,
            slab@.perms_wf(free_perms, prov),
    {
        Self::lemma_from_raw_parts_post_loop(slab, addr, len, total_num_blocks);
        Self::lemma_view_fields(slab);
        assert forall|i: int|
            #![trigger free_perms[i]]
            0 <= i < slab@.num_data_blocks implies free_perms[i].is_range(
            slab@.block_addr(i),
            slab@.block_size,
        ) && free_perms[i].provenance() == prov by {
            assert(slab@.block_addr(i) == slab.data_addr as int + i * slab.block_size as int);
        }
        lemma_fresh_slab_perms_wf(slab@, free_perms, prov);
    }

    /// Lemma: Establishes allocation postconditions and extracts the block's
    /// permission from the tracked permission map.
    proof fn lemma_alloc_take_block_perm(
        slab: &Slab,
        old_slab: &Slab,
        block: int,
        block_addr: int,
        tracked perms: &mut SlabPerms,
    ) -> (tracked result: PointsToRaw)
        requires
            old_slab.inv(),
            slab.inv(),
            block >= slab.num_index_blocks as int,
            block < (slab.num_index_blocks + slab.num_data_blocks) as int,
            slab.index@.is_bit_set(block),
            !old_slab.index@.is_bit_set(block),
            forall|k: int|
                k != block && 0 <= k < slab.index@.num_bits ==> slab.index@.is_bit_set(k)
                    == old_slab.index@.is_bit_set(k),
            slab.num_index_blocks == old_slab.num_index_blocks,
            slab.num_data_blocks == old_slab.num_data_blocks,
            slab.block_size == old_slab.block_size,
            slab.data_addr == old_slab.data_addr,
            slab.index@.num_bits == old_slab.index@.num_bits,
            block_addr == (slab.data_addr as int) + (block - slab.num_index_blocks as int) * (
            slab.block_size as int),
            old(perms).wf(old_slab@, old(perms).index_perm.provenance()),
        ensures
            ({
                let block_idx: int = block - slab.num_index_blocks as int;
                &&& old_slab@.is_valid_addr(block_addr)
                &&& old_slab@.addr_to_block_idx(block_addr) == block_idx
                &&& !old_slab@.is_allocated(block_idx)
                &&& slab@.is_allocated(block_idx)
                &&& slab@.num_data_blocks == old_slab@.num_data_blocks
                &&& slab@.block_size == old_slab@.block_size
                &&& slab@.data_addr == old_slab@.data_addr
                &&& slab@.allocated_blocks =~= old_slab@.allocated_blocks.insert(block_idx)
                &&& block_addr > 0
                &&& result.is_range(block_addr, old_slab@.block_size)
                &&& result.provenance() == old(perms).index_perm.provenance()
                &&& perms.wf(slab@, old(perms).index_perm.provenance())
                &&& perms.index_perm == old(perms).index_perm
            }),
    {
        let block_idx: int = block - slab.num_index_blocks as int;
        Self::lemma_alloc_establishes_postconditions(slab, old_slab, block, block_idx, block_addr);
        perms.take_block_perm(block_idx)
    }

    /// Lemma: Proves deallocation postconditions and restores the block's
    /// permission to the tracked permission map after a successful bitmap clear.
    proof fn lemma_dealloc_ok_finalize(
        slab: &Slab,
        old_slab: &Slab,
        index: int,
        ptr: int,
        tracked perms: &mut SlabPerms,
        tracked block_perm: PointsToRaw,
    )
        requires
            old_slab.inv(),
            slab.index.inv(),
            slab.index@.num_bits == old_slab.index@.num_bits,
            slab.num_index_blocks == old_slab.num_index_blocks,
            slab.num_data_blocks == old_slab.num_data_blocks,
            slab.block_size == old_slab.block_size,
            slab.data_addr == old_slab.data_addr,
            !slab.index@.is_bit_set(index),
            old_slab.index@.is_bit_set(index),
            forall|j: int|
                j != index && 0 <= j < slab.index@.num_bits ==> slab.index@.is_bit_set(j)
                    == old_slab.index@.is_bit_set(j),
            index == (old_slab.num_index_blocks as int) + old_slab@.addr_to_block_idx(ptr),
            old_slab@.is_valid_addr(ptr),
            old_slab@.can_deallocate(old_slab@.addr_to_block_idx(ptr)),
            block_perm.is_range(ptr, old_slab@.block_size),
            block_perm.provenance() == old(perms).index_perm.provenance(),
            old(perms).wf(old_slab@, old(perms).index_perm.provenance()),
        ensures
            slab.inv(),
            ({
                let block_idx: int = old_slab@.addr_to_block_idx(ptr);
                &&& !slab@.is_allocated(block_idx)
                &&& slab@.num_data_blocks == old_slab@.num_data_blocks
                &&& slab@.block_size == old_slab@.block_size
                &&& slab@.data_addr == old_slab@.data_addr
                &&& slab@.allocated_blocks =~= old_slab@.allocated_blocks.remove(block_idx)
                &&& slab@.can_allocate()
                &&& perms.wf(slab@, old(perms).index_perm.provenance())
                &&& perms.index_perm == old(perms).index_perm
            }),
    {
        Self::lemma_dealloc_clear_ok_with_perms(
            slab,
            old_slab,
            index,
            ptr,
            old(perms).free_perms,
            old(perms).index_perm.provenance(),
        );
        let block_idx: int = old_slab@.addr_to_block_idx(ptr);
        Self::lemma_block_addr_inverse(&old_slab@, ptr);
        lemma_dealloc_perms_wf(
            old_slab@,
            slab@,
            old(perms).free_perms,
            block_idx,
            block_perm,
            old(perms).index_perm.provenance(),
        );
        perms.put_block_perm(block_idx, block_perm);
    }

    /// Proves that u8 layout satisfies RawArray::from_raw_parts precondition.
    proof fn lemma_u8_layout_for_raw_array(index_len: usize, total_num_blocks: usize, len: usize)
        requires
            index_len == total_num_blocks / 8,
            total_num_blocks >= 8,
            index_len <= len,
        ensures
            vstd::layout::size_of::<u8>() == 1,
            vstd::layout::align_of::<u8>() == 1,
            index_len <= len,
    {
        assert(vstd::layout::align_of::<u8>() == 1) by {
            let a: nat = vstd::layout::align_of::<u8>();
            assert(a == 1) by (nonlinear_arith)
                requires
                    1nat % a == 0,
                    a != 0nat,
            ;
        }
    }

    /// Splits the initial memory permission into index and per-block data permissions.
    proof fn split_mem_into_slab_perms(
        tracked mem: PointsToRaw,
        addr: int,
        data_addr: int,
        len: int,
        block_size: int,
        total_num_blocks: int,
        num_index_blocks: int,
        num_data_blocks: int,
    ) -> (tracked result: (PointsToRaw, Map<int, PointsToRaw>))
        requires
            mem.is_range(addr, len),
            len > 0,
            block_size > 0,
            total_num_blocks == len / block_size,
            num_index_blocks >= 1,
            num_data_blocks > 0,
            num_index_blocks + num_data_blocks == total_num_blocks,
            data_addr == addr + num_index_blocks * block_size,
        ensures
            result.0.is_range(addr, num_index_blocks * block_size),
            result.0.provenance() == mem.provenance(),
            forall|i: int| 0 <= i < num_data_blocks <==> result.1.dom().contains(i),
            forall|i: int|
                #![trigger result.1[i]]
                0 <= i < num_data_blocks ==> result.1[i].is_range(
                    data_addr + i * block_size,
                    block_size,
                ) && result.1[i].provenance() == mem.provenance(),
    {
        let used_size: int = total_num_blocks * block_size;
        let used_range: Set<int> = set_int_range(addr, addr + used_size);
        Self::lemma_div_mul_le(len, block_size);
        assert forall|x: int| #![auto] used_range.contains(x) implies mem.dom().contains(x) by {}
        let tracked (used_perm, _padding) = mem.split(used_range);

        let idx_size: int = num_index_blocks * block_size;
        let index_range: Set<int> = set_int_range(addr, addr + idx_size);
        Self::lemma_distributive(num_index_blocks, num_data_blocks, block_size);
        assert(idx_size <= used_size) by {
            Self::lemma_mul_inequality(num_index_blocks, total_num_blocks, block_size);
        }
        assert forall|x: int| #![auto] index_range.contains(x) implies used_perm.dom().contains(
            x,
        ) by {}
        let tracked (idx_perm, data_perm) = used_perm.split(index_range);

        assert(data_perm.dom() =~= set_int_range(
            data_addr,
            data_addr + num_data_blocks * block_size,
        )) by {
            assert forall|x: int|
                used_perm.dom().difference(index_range).contains(x) <==> set_int_range(
                    data_addr,
                    data_addr + num_data_blocks * block_size,
                ).contains(x) by {}
        }

        let tracked fp = split_into_blocks(data_perm, data_addr, block_size, num_data_blocks);

        (idx_perm, fp)
    }
}

//==================================================================================================
// PointsToRaw Memory Permission Proof Helpers
//==================================================================================================
/// Lemma: The last block's range is a subset of the full range.
proof fn lemma_range_subset_last_block(base: int, block_size: int, n: int)
    requires
        block_size > 0,
        n >= 1,
    ensures
        set_int_range(base + (n - 1) * block_size, base + n * block_size).subset_of(
            set_int_range(base, base + n * block_size),
        ),
{
    let last: Set<int> = set_int_range(base + (n - 1) * block_size, base + n * block_size);
    let full: Set<int> = set_int_range(base, base + n * block_size);
    assert forall|x: int| last.contains(x) implies full.contains(x) by {
        assert((n - 1) * block_size >= 0) by (nonlinear_arith)
            requires
                n >= 1,
                block_size > 0,
        ;
    }
}

/// Lemma: Removing the last block from the full range gives the prefix range.
proof fn lemma_range_difference_last_block(base: int, block_size: int, n: int)
    requires
        block_size > 0,
        n >= 1,
    ensures
        set_int_range(base, base + n * block_size).difference(
            set_int_range(base + (n - 1) * block_size, base + n * block_size),
        ) =~= set_int_range(base, base + (n - 1) * block_size),
{
    let full: Set<int> = set_int_range(base, base + n * block_size);
    let last: Set<int> = set_int_range(base + (n - 1) * block_size, base + n * block_size);
    let prefix: Set<int> = set_int_range(base, base + (n - 1) * block_size);
    let diff: Set<int> = full.difference(last);

    assert forall|x: int| diff.contains(x) <==> prefix.contains(x) by {
        if diff.contains(x) {
        }
        if prefix.contains(x) {
            assert(base <= x < base + (n - 1) * block_size);
            assert((n - 1) * block_size <= n * block_size) by (nonlinear_arith)
                requires
                    block_size > 0,
                    n >= 1,
            ;
            assert(x < base + n * block_size);
            assert(full.contains(x));
            assert((n - 1) * block_size < n * block_size) by (nonlinear_arith)
                requires
                    block_size > 0,
                    n >= 1,
            ;
            assert(!last.contains(x));
        }
    }
}

/// Splits a contiguous PointsToRaw into per-block permissions.
/// Given a permission for [base, base + n * block_size), returns a Map mapping
/// block index i to a permission for [base + i * block_size, base + (i+1) * block_size).
proof fn split_into_blocks(
    tracked perm: PointsToRaw,
    base: int,
    block_size: int,
    n: int,
) -> (tracked result: Map<int, PointsToRaw>)
    requires
        perm.is_range(base, n * block_size),
        block_size > 0,
        n >= 0,
    ensures
        forall|i: int| 0 <= i < n <==> result.dom().contains(i),
        forall|i: int|
            #![trigger result[i]]
            0 <= i < n ==> result[i].is_range(base + i * block_size, block_size)
                && result[i].provenance() == perm.provenance(),
    decreases n,
{
    if n == 0 {
        let tracked _padding = perm;
        Map::<int, PointsToRaw>::tracked_empty()
    } else {
        // Split off the last block (index n-1).
        let last_start: int = base + (n - 1) * block_size;
        let last_range: Set<int> = set_int_range(last_start, last_start + block_size);

        assert((n - 1) * block_size + block_size == n * block_size) by (nonlinear_arith)
            requires
                block_size > 0,
                n >= 1,
        ;

        lemma_range_subset_last_block(base, block_size, n);

        let tracked (last_perm, rest_perm) = perm.split(last_range);

        lemma_range_difference_last_block(base, block_size, n);

        let tracked mut result = split_into_blocks(rest_perm, base, block_size, n - 1);
        result.tracked_insert(n - 1, last_perm);
        result
    }
}

/// Joins per-block permissions back into a contiguous PointsToRaw.
/// Inverse of split_into_blocks.
/// NOTE: Currently unused. Reserved for future slab destruction / memory reclamation.
proof fn join_block_perms(
    tracked perms: Map<int, PointsToRaw>,
    base: int,
    block_size: int,
    n: int,
    prov: Provenance,
) -> (tracked result: PointsToRaw)
    requires
        block_size > 0,
        n >= 0,
        forall|i: int| 0 <= i < n <==> perms.dom().contains(i),
        forall|i: int|
            #![trigger perms[i]]
            0 <= i < n ==> perms[i].is_range(base + i * block_size, block_size)
                && perms[i].provenance() == prov,
    ensures
        result.is_range(base, n * block_size),
        result.provenance() == prov,
    decreases n,
{
    if n == 0 {
        assert(n * block_size == 0) by (nonlinear_arith)
            requires
                n == 0,
        ;
        PointsToRaw::empty(prov)
    } else {
        let tracked mut perms = perms;
        let tracked last_perm = perms.tracked_remove(n - 1);
        let tracked prefix_perm = join_block_perms(perms, base, block_size, n - 1, prov);
        let tracked result = prefix_perm.join(last_perm);

        let last_start: int = base + (n - 1) * block_size;
        assert((n - 1) * block_size + block_size == n * block_size) by (nonlinear_arith)
            requires
                block_size > 0,
                n >= 1,
        ;

        assert(set_int_range(base, base + (n - 1) * block_size) + set_int_range(
            last_start,
            last_start + block_size,
        ) =~= set_int_range(base, base + n * block_size)) by {
            let prefix_set: Set<int> = set_int_range(base, base + (n - 1) * block_size);
            let last_set: Set<int> = set_int_range(last_start, last_start + block_size);
            let full_set: Set<int> = set_int_range(base, base + n * block_size);
            assert forall|x: int|
                #![trigger full_set.contains(x)]
                (prefix_set + last_set).contains(x) <==> full_set.contains(x) by {
                assert((n - 1) * block_size >= 0) by (nonlinear_arith)
                    requires
                        n >= 1,
                        block_size > 0,
                ;
            }
        }

        result
    }
}

/// Lemma: After deallocating block_idx and inserting its permission back,
/// the map is well-formed for the new slab state.
proof fn lemma_dealloc_perms_wf(
    old_view: SlabView,
    new_view: SlabView,
    perms: Map<int, PointsToRaw>,
    block_idx: int,
    block_perm: PointsToRaw,
    prov: Provenance,
)
    requires
        old_view.block_size > 0,
        old_view.num_data_blocks > 0,
        old_view.perms_wf(perms, prov),
        0 <= block_idx < old_view.num_data_blocks,
        old_view.is_allocated(block_idx),
        block_perm.is_range(old_view.block_addr(block_idx), old_view.block_size),
        block_perm.provenance() == prov,
        new_view.num_data_blocks == old_view.num_data_blocks,
        new_view.block_size == old_view.block_size,
        new_view.data_addr == old_view.data_addr,
        new_view.allocated_blocks =~= old_view.allocated_blocks.remove(block_idx),
    ensures
        new_view.perms_wf(perms.insert(block_idx, block_perm), prov),
{
    let new_perms: Map<int, PointsToRaw> = perms.insert(block_idx, block_perm);

    // Prove new_view.perms_wf(new_perms, prov).
    // For free blocks in new_view:
    // For allocated blocks in new_view:
    assert forall|i: int|
        #![trigger new_perms.dom().contains(i)]
        (0 <= i < new_view.num_data_blocks && !new_view.is_allocated(
            i,
        )) implies new_perms.dom().contains(i) && (#[trigger] new_perms[i]).is_range(
        new_view.block_addr(i),
        new_view.block_size,
    ) && new_perms[i].provenance() == prov by {
        if i == block_idx {
        } else {
        }
    }
    assert forall|i: int|
        #![trigger new_perms.dom().contains(i)]
        (0 <= i < new_view.num_data_blocks && new_view.is_allocated(
            i,
        )) implies !new_perms.dom().contains(i) by {}
}

proof fn lemma_fresh_slab_perms_wf(view: SlabView, perms: Map<int, PointsToRaw>, prov: Provenance)
    requires
        view.block_size > 0,
        view.num_data_blocks > 0,
        view.is_freshly_initialized(),
        forall|i: int| 0 <= i < view.num_data_blocks <==> perms.dom().contains(i),
        forall|i: int|
            #![trigger perms[i]]
            0 <= i < view.num_data_blocks ==> perms[i].is_range(view.block_addr(i), view.block_size)
                && perms[i].provenance() == prov,
    ensures
        view.perms_wf(perms, prov),
{
    assert forall|i: int|
        #![trigger perms.dom().contains(i)]
        (0 <= i < view.num_data_blocks && !view.is_allocated(i)) implies perms.dom().contains(i)
        && (#[trigger] perms[i]).is_range(view.block_addr(i), view.block_size)
        && perms[i].provenance() == prov by {}
    assert forall|i: int|
        #![trigger perms.dom().contains(i)]
        (0 <= i < view.num_data_blocks && view.is_allocated(i)) implies !perms.dom().contains(
        i,
    ) by {}
}
} // verus!