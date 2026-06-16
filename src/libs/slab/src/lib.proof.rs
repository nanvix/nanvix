// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Slab - Proofs
//
// This file contains lemmas and proof functions for Slab.

verus! {

impl View for Slab {
    type V = SlabView;

    closed spec fn view(&self) -> SlabView {
        let bitmap_view = self.index@;
        let data_addr_as_int = self.data_addr as usize as int;
        let set_bits = Set::<int>::new(|i: int| 0 <= i < self.num_data_blocks() && bitmap_view.is_bit_set(i));
        let free_bits = Set::<int>::new(|i: int| 0 <= i < self.num_data_blocks() && !bitmap_view.is_bit_set(i));
        SlabView {
            block_size: self.block_size,
            start_addr: self.data_addr as usize,
            end_addr: self.end_addr as usize,
            allocated_addrs: set_bits.map(|i: int| (data_addr_as_int + i * self.block_size) as usize),
            free_addrs: free_bits.map(|i: int| (data_addr_as_int + i * self.block_size) as usize),
        }
    }
}

impl Slab {
    pub open spec fn inv(&self) -> bool {
        &&& self@.inv()
        &&& self.internal_inv()
    }

    pub closed spec fn num_data_blocks(&self) -> usize {
        ((self.end_addr as usize - self.data_addr as usize) / (self.block_size as int)) as usize
    }

    pub closed spec fn internal_inv(&self) -> bool {
        &&& self.block_size > 0
        &&& self.index.inv()
        &&& self.index@.num_bits >= self.num_data_blocks()
        &&& (self.data_addr as usize) < (self.end_addr as usize) <= usize::MAX
        &&& self.data_addr as usize % self.block_size == 0
        &&& self.block_size > 0
        &&& (self.end_addr as usize - self.data_addr as usize) % (self.block_size as int) == 0
        &&& (self.end_addr as usize - self.data_addr as usize) <= isize::MAX
        &&& forall|i: int| self.num_data_blocks() as int <= i < self.index@.num_bits as int
                ==> self.index@.is_bit_set(i)
    }

    proof fn lemma_wrapping_add_consequences(
        addr: *mut u8,
        len: usize
    )
        ensures
            if ((addr as usize) + (len * size_of::<u8>())) % (usize::MAX + 1) < (addr as usize) {
                (addr as usize) + len > usize::MAX
            }
            else {
                (addr as usize) + len <= usize::MAX
            },
    {
        assert(
            if ((addr as usize) + (len * size_of::<u8>())) % (usize::MAX + 1) < (addr as usize) {
                (addr as usize) + len > usize::MAX
            }
            else {
                (addr as usize) + len <= usize::MAX
            }
        ) by (nonlinear_arith)
            requires
                size_of::<u8>() == 1,
        ;
    }

    proof fn lemma_no_room_for_index(
        len: usize,
        block_size: usize,
        total_num_blocks: usize,
        num_index_blocks: usize,
        divisor: usize,
    )
        requires
            len <= isize::MAX,
            block_size > 0,
            total_num_blocks == len / block_size,
            divisor == block_size * (u8::BITS as usize) + 1,
            num_index_blocks == (total_num_blocks / divisor)
                + if total_num_blocks % divisor == 0 {
                      0int
                  } else {
                      1int
                  },
            num_index_blocks >= total_num_blocks,
        ensures
            len < block_size * 2,
    {
        assert(len < block_size * 2) by (nonlinear_arith)
            requires
                len <= isize::MAX,
                block_size > 0,
                total_num_blocks == len / block_size,
                u8::BITS == 8,
                divisor == block_size * u8::BITS + 1,
                num_index_blocks == (total_num_blocks / divisor)
                    + if total_num_blocks % divisor == 0 {
                          0int
                      } else {
                          1int
                      },
                num_index_blocks >= total_num_blocks,
        ;
    }

    proof fn lemma_can_compute_data_addr(
        addr: *mut u8,
        total_num_blocks: usize,
        num_index_blocks: usize,
        block_size: usize,
        len: usize
    )
        requires
            ((addr as usize) + len * size_of::<u8>()) % (usize::MAX + 1) >= addr as usize,
            len <= isize::MAX,
            block_size > 0,
            total_num_blocks == len / block_size,
            num_index_blocks < total_num_blocks,
        ensures
            num_index_blocks * block_size < total_num_blocks * block_size <= len,
            (addr as usize) + (total_num_blocks * block_size) * size_of::<u8>()
                + vstd::layout::align_of::<u8>() - 1 <= usize::MAX,
            (total_num_blocks * block_size) * size_of::<u8>() <= isize::MAX,
            (num_index_blocks * block_size) * size_of::<u8>() <= isize::MAX,
            (addr as usize) + len * size_of::<u8>() <= usize::MAX,
    {
        axiom_align_of_u8_is_1();
        assert(size_of::<u8>() == 1);
        assert(num_index_blocks * block_size < total_num_blocks * block_size <= len) by (nonlinear_arith)
            requires
                block_size > 0,
                total_num_blocks == len / block_size,
                num_index_blocks < total_num_blocks,
        ;
        assert(num_index_blocks * block_size * size_of::<u8>() == num_index_blocks * block_size);
        assert((addr as usize) + (total_num_blocks * block_size) * size_of::<u8>() ==
               (addr as usize) + (total_num_blocks * block_size));
        assert(len * 1 == len);
        assert((addr as usize) + len <= usize::MAX) by (nonlinear_arith)
            requires
                ((addr as usize) + len) % (usize::MAX + 1) >= addr as usize,
        ;
        assert((addr as usize) + (total_num_blocks * block_size) * size_of::<u8>() <= usize::MAX);
        assert((total_num_blocks * block_size) * size_of::<u8>() <= isize::MAX) by (nonlinear_arith)
            requires
                total_num_blocks * block_size <= len,
                len <= isize::MAX,
                size_of::<u8>() == 1,
        ;
    }

    proof fn lemma_can_create_raw_array(
        addr: *mut u8,
        total_num_blocks: usize,
        num_index_blocks: usize,
        num_data_blocks: usize,
        block_size: usize,
        len: usize,
        index_len: usize,
    )
        requires
            ((addr as usize) + len * size_of::<u8>()) % (usize::MAX + 1) >= addr as usize,
            block_size > 0,
            total_num_blocks == len / block_size,
            num_index_blocks < total_num_blocks,
            num_data_blocks == total_num_blocks - num_index_blocks,
            index_len == (num_data_blocks / (u8::BITS as usize))
                + if num_data_blocks.is_multiple_of(u8::BITS as usize) {
                    0int
                } else {
                    1int
                },
        ensures
            addr as usize + index_len * size_of::<u8>() + align_of::<u8>() - 1 <= usize::MAX,
    {
        axiom_align_of_u8_is_1();
        assert(size_of::<u8>() == 1);
        assert(addr as usize + index_len * size_of::<u8>() + align_of::<u8>() - 1 == addr as usize + index_len);
        assert((addr as usize) + len <= usize::MAX) by (nonlinear_arith)
            requires
                ((addr as usize) + len * size_of::<u8>()) % (usize::MAX + 1) >= addr as usize,
        ;
    }

    /// Proves that a freshly constructed Slab satisfies its invariant.
    proof fn lemma_from_raw_parts_establishes_inv(
        block_size: usize,
        data_addr: *mut u8,
        end_addr: *const u8,
        index: &Bitmap,
        addr: *mut u8,
        len: usize,
        total_num_blocks: usize,
        num_index_blocks: usize,
        num_data_blocks: usize,
        index_len: usize,
        U8_BITS: usize,
    )
        requires
            U8_BITS == u8::BITS as usize,
            len <= isize::MAX,
            block_size > 0,
            block_size < i32::MAX,
            num_data_blocks >= 1,
            num_data_blocks == total_num_blocks - num_index_blocks,
            num_index_blocks < total_num_blocks,
            total_num_blocks == len / block_size,
            data_addr as usize == addr as usize + num_index_blocks * block_size,
            end_addr as usize == addr as usize + total_num_blocks * block_size,
            addr as usize % block_size == 0,
            addr as usize + len * size_of::<u8>() <= usize::MAX,
            index.inv(),
            index@.num_bits == index_len * U8_BITS,
            index@.set_bits == Set::new(|j: int| num_data_blocks <= j < index_len * U8_BITS),
            index_len == (num_data_blocks / U8_BITS) + if num_data_blocks % U8_BITS == 0 { 0int } else { 1int },
        ensures
            ({
                let slab = Slab { index: *index, data_addr, end_addr, block_size };
                &&& slab@.block_size == slab.block_size
                &&& slab@.start_addr >= addr as usize
                &&& slab@.end_addr <= addr as usize + len
                &&& slab@.allocated_addrs == Set::<usize>::empty()
                &&& slab.inv()
                &&& forall|i: int| 0 <= i < (slab@.end_addr - slab@.start_addr) / block_size as int
                    ==> #[trigger] slab@.free_addrs.contains(
                        (slab@.start_addr + i * block_size as int) as usize)
            })
    {
        assert(size_of::<u8>() == 1);

        let slab = Slab { index: *index, data_addr, end_addr, block_size };

        // Prove internal_inv().

        vstd::arithmetic::mul::lemma_mul_is_commutative(total_num_blocks as int, slab.block_size as int);

        assert(total_num_blocks * slab.block_size <= len) by {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(len as int, slab.block_size as int);
        }

        assert(num_index_blocks * slab.block_size + num_data_blocks * slab.block_size ==
               total_num_blocks * slab.block_size) by {
            vstd::arithmetic::mul::lemma_mul_is_distributive_add_other_way(
                slab.block_size as int,
                num_index_blocks as int,
                num_data_blocks as int
            );
        }

        assert(slab.data_addr as usize % slab.block_size == 0) by {
            vstd::arithmetic::div_mod::lemma_mod_multiples_vanish(
                num_index_blocks as int, addr as usize as int, slab.block_size as int
            );
            vstd::arithmetic::mul::lemma_mul_is_commutative(
                num_index_blocks as int, slab.block_size as int
            );
        }

        // Establish that end_addr - data_addr == num_data_blocks * block_size.
        assert(slab.end_addr as usize - slab.data_addr as usize == num_data_blocks * slab.block_size);

        // Establish num_data_blocks() == num_data_blocks (needed for choose witnesses below).
        assert(slab.num_data_blocks() == num_data_blocks) by {
            vstd::arithmetic::div_mod::lemma_div_by_multiple(
                num_data_blocks as int, slab.block_size as int,
            );
            vstd::arithmetic::mul::lemma_mul_is_commutative(
                num_data_blocks as int, slab.block_size as int,
            );
        }

        // (end_addr - data_addr) == num_data_blocks * block_size, and block_size divides it.
        assert(slab.end_addr as usize - slab.data_addr as usize == num_data_blocks * slab.block_size);
        assert((slab.end_addr as usize - slab.data_addr as usize) % (slab.block_size as int) == 0) by {
            vstd::arithmetic::div_mod::lemma_mod_multiples_basic(
                num_data_blocks as int, slab.block_size as int,
            );
            vstd::arithmetic::mul::lemma_mul_is_commutative(
                num_data_blocks as int, slab.block_size as int,
            );
        }

        // data_addr < end_addr (since num_data_blocks >= 1 and block_size > 0).
        assert(num_data_blocks * slab.block_size > 0) by (nonlinear_arith)
            requires num_data_blocks >= 1int, slab.block_size >= 1;
        assert((slab.data_addr as usize) < (slab.end_addr as usize));

        // (end_addr - data_addr) <= isize::MAX.
        assert((slab.end_addr as usize - slab.data_addr as usize) <= isize::MAX);

        assert(slab.internal_inv());

        // Prove slab@.inv() (SlabView::inv()).

        assert(slab@.end_addr % slab@.block_size == 0) by {
            vstd::arithmetic::div_mod::lemma_mod_multiples_vanish(
                slab.num_data_blocks() as int,
                slab.data_addr as usize as int,
                slab.block_size as int,
            );
            vstd::arithmetic::mul::lemma_mul_is_commutative(
                slab.num_data_blocks() as int, slab.block_size as int,
            );
        }

        assert(slab@.end_addr > slab@.start_addr) by {
            assert(num_data_blocks * slab.block_size > 0) by (nonlinear_arith)
                requires
                    num_data_blocks >= 1,
                    slab.block_size >= 1,
            ;
        }

        // After construction, no bits < num_data_blocks are set, so allocated_addrs is empty.
        assert(slab.index@.set_bits =~= Set::new(|k: int| num_data_blocks <= k < index_len * U8_BITS));
        assert forall|i: int| slab.num_data_blocks() as int <= i < slab.index@.num_bits as int
            implies slab.index@.is_bit_set(i) by {}
        assert forall|j: int| 0 <= j < slab.num_data_blocks() implies !slab.index@.is_bit_set(j) by {}
        assert(slab@.allocated_addrs =~= Set::<usize>::empty());

        assert forall|a: usize| slab@.allocated_addrs.contains(a) implies
            slab@.start_addr <= a < slab@.end_addr && a as usize % slab@.block_size == 0 by {}

        assert forall|a: usize| slab@.free_addrs.contains(a) implies
            slab@.start_addr <= a < slab@.end_addr && a % slab@.block_size == 0 by {
            let data_addr_as_int = slab.data_addr as usize as int;
            let free_bits = Set::<int>::new(|i: int| 0 <= i < slab.num_data_blocks() && !slab.index@.is_bit_set(i));
            let j = choose|j: int| free_bits.contains(j) && a == (data_addr_as_int + j * slab.block_size) as usize;
            assert(j * slab.block_size < num_data_blocks * slab.block_size) by (nonlinear_arith)
                requires 0 <= j < num_data_blocks as int, slab.block_size >= 1;
            // Overflow bound: data_addr + j * bs < data_addr + num_data_blocks * bs <= usize::MAX.
            assert(num_index_blocks * slab.block_size + num_data_blocks * slab.block_size ==
                   total_num_blocks * slab.block_size) by {
                vstd::arithmetic::mul::lemma_mul_is_distributive_add_other_way(
                    slab.block_size as int, num_index_blocks as int, num_data_blocks as int);
            }
            assert(total_num_blocks * slab.block_size <= len) by {
                vstd::arithmetic::div_mod::lemma_fundamental_div_mod(len as int, slab.block_size as int);
            }
            assert(a % slab.block_size == 0) by {
                vstd::arithmetic::mul::lemma_mul_is_commutative(j, slab.block_size as int);
                vstd::arithmetic::div_mod::lemma_mod_multiples_vanish(j, data_addr_as_int, slab.block_size as int);
            }
        }

        assert(slab@.allocated_addrs.disjoint(slab@.free_addrs));

        // Prove that every valid block index maps to a free address.
        assert forall|i: int| 0 <= i < (slab@.end_addr - slab@.start_addr) / block_size as int
            implies #[trigger] slab@.free_addrs.contains(
                (slab@.start_addr + i * block_size as int) as usize) by {
            assert(!slab.index@.is_bit_set(i));
            let free_bits = Set::<int>::new(|j: int| 0 <= j < slab.num_data_blocks() && !slab.index@.is_bit_set(j));
            assert(free_bits.contains(i));
        }

        // Completeness: every block-aligned address in [start_addr, end_addr) is in free_addrs.
        // (After construction allocated_addrs is empty, so they must all be free.)
        assert forall|a: usize| a % slab@.block_size == 0 && slab@.start_addr <= a < slab@.end_addr
            implies slab@.allocated_addrs.contains(a) || slab@.free_addrs.contains(a) by {
            let data_addr_as_int = slab.data_addr as usize as int;
            let bs = slab.block_size as int;
            let diff = a as int - data_addr_as_int;
            // diff >= 0 and diff % bs == 0
            assert(diff >= 0);
            vstd::arithmetic::div_mod::lemma_sub_mod_noop(a as int, data_addr_as_int, bs);
            // lemma gives: ((a%bs) - (data_addr%bs)) % bs == diff % bs
            // a%bs==0, data_addr%bs==0, so (0-0)%bs == 0
            assert((0 as int) % bs == 0) by (nonlinear_arith) requires bs > 0;
            assert(diff % bs == 0);
            let j = diff / bs;
            // j >= 0
            vstd::arithmetic::div_mod::lemma_div_is_ordered(0, diff, bs);
            // j < num_data_blocks: diff < num_data_blocks * bs, so diff/bs < num_data_blocks
            assert(diff < num_data_blocks as int * bs);
            assert(j < num_data_blocks as int) by (nonlinear_arith)
                requires diff >= 0, diff < num_data_blocks as int * bs, bs > 0, diff % bs == 0, j == diff / bs;
            // bit j is unset (all bits < num_data_blocks are unset after construction)
            assert(!slab.index@.is_bit_set(j));
            // a == (data_addr + j * bs) as usize
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(diff, bs);
            assert(diff == j * bs);
            assert(a == (data_addr_as_int + j * bs) as usize);
            // So a is in free_addrs
            let free_bits = Set::<int>::new(|i: int| 0 <= i < slab.num_data_blocks() && !slab.index@.is_bit_set(i));
            assert(free_bits.contains(j));
        }
    }

    /// Proves that the address mapping is injective: distinct block indices
    /// within [0, num_data_blocks) produce distinct addresses.
    proof fn lemma_addr_injective(
        data_addr: *mut u8,
        num_data_blocks: usize,
        block_size: usize,
        block: usize,
    )
        requires
            block_size > 0,
            block < num_data_blocks,
            data_addr as usize + num_data_blocks * block_size <= usize::MAX,
        ensures
            forall|i: int|
                0 <= i < (num_data_blocks as int) && i != (block as int)
                ==> #[trigger] ((data_addr as usize as int + i * (block_size as int)) as usize)
                    != ((data_addr as usize as int + (block as int) * (block_size as int)) as usize),
    {
        let da = data_addr as usize as int;
        let bs = block_size as int;
        let ndb = num_data_blocks as int;
        let bi = block as int;
        assert forall|i: int|
            0 <= i < ndb && i != bi
            implies
            #[trigger] ((da + i * bs) as usize) != ((da + bi * bs) as usize) by {
            assert(i * bs != bi * bs)
                by (nonlinear_arith) requires i != bi, bs > 0;
            assert(i * bs < ndb * bs)
                by (nonlinear_arith) requires 0 <= i, i < ndb, bs > 0;
            assert(bi * bs < ndb * bs)
                by (nonlinear_arith) requires 0 <= bi, bi < ndb, bs > 0;
        }
    }

    /// Proves that a set bit within the data range implies the
    /// corresponding address is in `allocated_addrs`.
    proof fn lemma_set_bit_implies_allocated(&self, i: int)
        requires
            self.internal_inv(),
            self.block_size > 0,
            0 <= i < self.num_data_blocks(),
            self.index@.is_bit_set(i),
        ensures
            self@.allocated_addrs.contains(
                (self.data_addr as usize as int + i * self.block_size) as usize,
            ),
    {
        assert(Set::<int>::new(
            |j: int| 0 <= j < self.num_data_blocks() && self.index@.is_bit_set(j)
        ).contains(i));
    }

    /// Proves that an unset bit within the data range implies the
    /// corresponding address is in `free_addrs`.
    proof fn lemma_unset_bit_implies_free(&self, i: int)
        requires
            self.internal_inv(),
            self.block_size > 0,
            0 <= i < self.num_data_blocks(),
            !self.index@.is_bit_set(i),
        ensures
            self@.free_addrs.contains(
                (self.data_addr as usize as int + i * self.block_size) as usize,
            ),
    {
        assert(Set::<int>::new(
            |j: int| 0 <= j < self.num_data_blocks() && !self.index@.is_bit_set(j)
        ).contains(i));
    }

    /// Proves that membership in `allocated_addrs` implies a set bit.
    proof fn lemma_allocated_implies_set_bit(&self, a: usize) -> (i: int)
        requires
            self.internal_inv(),
        ensures
            self@.allocated_addrs.contains(a) ==> {
                &&& 0 <= i < self.num_data_blocks()
                &&& self.index@.is_bit_set(i)
                &&& a == (self.data_addr as usize as int + i * self.block_size) as usize
            },
    {
        if self@.allocated_addrs.contains(a) {
            choose|i: int| 0 <= i < self.num_data_blocks()
                && self.index@.is_bit_set(i)
                && a == (self.data_addr as usize as int + i * self.block_size) as usize
        }
        else {
            0
        }
    }

    /// Proves that membership in `free_addrs` implies an unset bit.
    proof fn lemma_free_implies_unset_bit(&self, a: usize) -> (i: int)
        requires
            self.internal_inv(),
            self@.free_addrs.contains(a),
        ensures
            0 <= i < self.num_data_blocks(),
            !self.index@.is_bit_set(i),
            a == (self.data_addr as usize as int + i * self.block_size) as usize,
    {
        choose|i: int| 0 <= i < self.num_data_blocks()
            && !self.index@.is_bit_set(i)
            && a == (self.data_addr as usize as int + i * self.block_size) as usize
    }

    /// Proves that the pointer addition in `allocate` is OK to do.
    proof fn lemma_allocate_add_is_safe(
        &self,
        block: usize,
    )
        requires
            self.block_size > 0,
            block < (self.end_addr as usize - self.data_addr as usize) / (self.block_size as int),
            (self.end_addr as usize - self.data_addr as usize) % (self.block_size as int) == 0,
         ensures
            self.data_addr as usize + block * self.block_size < self.end_addr as usize,
            size_of::<u8>() == 1,
    {
        assert(self.data_addr as usize + block * self.block_size < self.end_addr as usize) by (nonlinear_arith)
            requires
                self.block_size > 0,
                block < (self.end_addr as usize - self.data_addr as usize) / (self.block_size as int),
                (self.end_addr as usize - self.data_addr as usize) % (self.block_size as int) == 0,
           ;
    }

    /// Proves the postconditions of `allocate` in the success case:
    /// the returned address was free, allocated_addrs gains it,
    /// free_addrs loses it, and the invariant is preserved.
    proof fn lemma_allocate_ok(
        self: &Slab,
        old_self: &Slab,
        block: usize,
        addr: usize,
    )
        requires
            old_self.inv(),
            // Bitmap alloc postconditions.
            self.index.inv(),
            0 <= block < old_self.index@.num_bits,
            !old_self.index@.is_bit_set(block as int),
            self.index@.is_bit_set(block as int),
            self.index@.num_bits == old_self.index@.num_bits,
            forall|i: int| 0 <= i < self.index@.num_bits && i != block as int
                ==> self.index@.is_bit_set(i) == old_self.index@.is_bit_set(i),
            // Frame: non-index fields unchanged.
            self.data_addr == old_self.data_addr,
            self.num_data_blocks() == old_self.num_data_blocks(),
            self.block_size == old_self.block_size,
            self.end_addr == old_self.end_addr,
            // block < num_data_blocks (from sentinel invariant).
            block < old_self.num_data_blocks(),
            // addr == data_addr + block * block_size.
            addr == old_self.data_addr as usize + block * old_self.block_size,
        ensures
            self.inv(),
            old_self@.free_addrs.contains(addr),
            self@.allocated_addrs == old_self@.allocated_addrs.insert(addr),
            self@.free_addrs == old_self@.free_addrs.remove(addr),
            self@.block_size == old_self@.block_size,
            self@.start_addr == old_self@.start_addr,
            self@.end_addr == old_self@.end_addr,
    {
        let bi = block as int;

        old_self.lemma_unset_bit_implies_free(bi);

        // Establish self.internal_inv() so we can call lemmas on self.
        // The sentinel bits (>= num_data_blocks) are unchanged since block < num_data_blocks.
        assert forall|i: int| self.num_data_blocks() as int <= i < self.index@.num_bits as int
            implies self.index@.is_bit_set(i) by {
            assert(i != block as int);
        };

        // Pre-compute overflow bound needed inside the quantifier and for lemma_addr_injective.
        assert(self.data_addr as usize + self.num_data_blocks() * self.block_size <= usize::MAX) by {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(
                (self.end_addr as usize - self.data_addr as usize) as int, self.block_size as int
            );
            vstd::arithmetic::mul::lemma_mul_is_commutative(
                self.num_data_blocks() as int, self.block_size as int
            );
        }

        assert(self.internal_inv());

        self.lemma_set_bit_implies_allocated(bi);

        // allocated_addrs == old.insert(addr).
        assert forall|a: usize|
            #[trigger] self@.allocated_addrs.contains(a)
                || old_self@.allocated_addrs.insert(addr).contains(a)
            implies
            self@.allocated_addrs.contains(a)
                && old_self@.allocated_addrs.insert(addr).contains(a) by {
            if self@.allocated_addrs.contains(a) {
                let j = self.lemma_allocated_implies_set_bit(a);
                if j != bi { old_self.lemma_set_bit_implies_allocated(j); }
            }
            if old_self@.allocated_addrs.insert(addr).contains(a) && a != addr {
                let j = old_self.lemma_allocated_implies_set_bit(a);
                self.lemma_set_bit_implies_allocated(j);
            }
        }
        assert(self@.allocated_addrs =~= old_self@.allocated_addrs.insert(addr));

        // free_addrs == old.remove(addr).
        Slab::lemma_addr_injective(
            self.data_addr, self.num_data_blocks(), self.block_size, block,
        );
        assert forall|a: usize|
            #[trigger] self@.free_addrs.contains(a)
                || old_self@.free_addrs.remove(addr).contains(a)
            implies
            self@.free_addrs.contains(a)
                && old_self@.free_addrs.remove(addr).contains(a) by {
            if self@.free_addrs.contains(a) {
                let j = self.lemma_free_implies_unset_bit(a);
                old_self.lemma_unset_bit_implies_free(j);
            }
            if old_self@.free_addrs.remove(addr).contains(a) {
                let j = old_self.lemma_free_implies_unset_bit(a);
                self.lemma_unset_bit_implies_free(j);
            }
        }
        assert(self@.free_addrs =~= old_self@.free_addrs.remove(addr));
    }

    /// Proves the precondition of `offset_from_unsigned` for an allocated address:
    /// the offset from `data_addr` to the address fits in `isize`.
    proof fn lemma_deallocate_offset_bound(&self, ptr: *const u8)
        requires
            self.inv(),
            ptr as usize >= self.data_addr as usize,
        ensures
            (ptr as usize - self.data_addr as usize) % (size_of::<u8>() as int) == 0,
            self@.allocated_addrs.contains(ptr as usize) ==> ptr as usize - self.data_addr as usize <= isize::MAX,
    {
        let addr = ptr as usize;
        if self@.allocated_addrs.contains(addr) {
            let block_idx = self.lemma_allocated_implies_set_bit(addr);
            assert(block_idx * self.block_size < self.num_data_blocks() * self.block_size) by (nonlinear_arith)
                requires 0 <= block_idx, block_idx < self.num_data_blocks(), self.block_size > 0;
        }
    }

    /// Proves that the index computed as `(ptr - data_addr) / block_size`
    /// is in range, has its bit set, and maps back to `addr`.
    proof fn lemma_deallocate_index_ok(&self, ptr: *const u8, index: usize)
        requires
            self.inv(),
            ptr as usize >= self.data_addr as usize,
            (ptr as usize) < self.end_addr as usize,
            index == (ptr as usize - self.data_addr as usize) / (self.block_size as int) / (size_of::<u8>() as int),
            (ptr as usize) % self.block_size == 0,
        ensures
            index < self.num_data_blocks(),
            ptr as usize == self.data_addr as usize + index * self.block_size,
            self@.allocated_addrs.contains(ptr as usize) ==> self.index@.is_bit_set(index as int),
    {
        assert(index == (ptr as usize - self.data_addr as usize) / (self.block_size as int)) by (nonlinear_arith)
            requires
                index == (ptr as usize - self.data_addr as usize) / (self.block_size as int) / 1,
        ;

        let addr = ptr as usize;
        // Step 1: Prove (addr - data_addr) % block_size == 0.
        assert((addr - self.data_addr as usize) % self.block_size as int == 0) by {
            let q = self.data_addr as usize as int / self.block_size as int;
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(self.data_addr as int, self.block_size as int);
            vstd::arithmetic::mul::lemma_mul_is_commutative(self.block_size as int, q);
            vstd::arithmetic::div_mod::lemma_mod_multiples_vanish(-q, addr as int, self.block_size as int);
            vstd::arithmetic::mul::lemma_mul_unary_negation(q as int, self.block_size as int);
        }

        // Step 2: From div/mod and remainder == 0, derive addr == data_addr + index * block_size.
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod(addr - self.data_addr as usize, self.block_size as int);
        vstd::arithmetic::mul::lemma_mul_is_commutative(index as int, self.block_size as int);

        // Step 3: From addr < end_addr, derive index < num_data_blocks.
        // end_addr - data_addr == num_data_blocks() * block_size (from internal_inv divisibility).
        let bs = self.block_size as int;
        let ndb = self.num_data_blocks() as int;
        let idx = index as int;
        // addr == data_addr + index * block_size, addr < end_addr == data_addr + ndb * bs.
        assert(ndb * bs == self.end_addr as usize - self.data_addr as usize) by {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(
                (self.end_addr as usize - self.data_addr as usize) as int, self.block_size as int
            );
            vstd::arithmetic::mul::lemma_mul_is_commutative(ndb, bs);
        }
        assert(idx * bs < ndb * bs);
        assert(idx < ndb) by (nonlinear_arith)
            requires idx * bs < ndb * bs, bs > 0;

        if self@.allocated_addrs.contains(addr) {
            let block_idx = self.lemma_allocated_implies_set_bit(addr);
            let bi = block_idx;
            assert(bi * bs < ndb * bs) by (nonlinear_arith)
                requires 0 <= bi, bi < ndb, bs > 0;
            vstd::arithmetic::div_mod::lemma_div_by_multiple(block_idx, self.block_size as int);
            vstd::arithmetic::mul::lemma_mul_is_commutative(block_idx, self.block_size as int);
        }
    }

    /// Proves the postconditions of `deallocate` in the success case:
    /// the given address was allocated, allocated_addrs loses it,
    /// free_addrs gains it, and the invariant is preserved.
    proof fn lemma_deallocate_ok(
        self: &Slab,
        old_self: &Slab,
        block: usize,
        ptr: *const u8,
    )
        requires
            old_self.inv(),
            // Bitmap clear postconditions.
            self.index.inv(),
            0 <= block < old_self.index@.num_bits,
            old_self.index@.is_bit_set(block as int),
            !self.index@.is_bit_set(block as int),
            self.index@.num_bits == old_self.index@.num_bits,
            forall|i: int| 0 <= i < self.index@.num_bits && i != block as int
                ==> self.index@.is_bit_set(i) == old_self.index@.is_bit_set(i),
            // Frame: non-index fields unchanged.
            self.data_addr == old_self.data_addr,
            self.num_data_blocks() == old_self.num_data_blocks(),
            self.block_size == old_self.block_size,
            self.end_addr == old_self.end_addr,
            // block < num_data_blocks (from sentinel invariant).
            block < old_self.num_data_blocks(),
            // ptr == data_addr + block * block_size.
            ptr as usize == old_self.data_addr as usize + block * old_self.block_size,
        ensures
            self.inv(),
            old_self@.allocated_addrs.contains(ptr as usize),
            self@.allocated_addrs == old_self@.allocated_addrs.remove(ptr as usize),
            self@.free_addrs == old_self@.free_addrs.insert(ptr as usize),
            self@.block_size == old_self@.block_size,
            self@.start_addr == old_self@.start_addr,
            self@.end_addr == old_self@.end_addr,
    {
        let addr = ptr as usize;
        let bi = block as int;

        old_self.lemma_set_bit_implies_allocated(bi);

        // Establish self.internal_inv() so we can call lemmas on self.
        // The sentinel bits (>= num_data_blocks) are unchanged since block < num_data_blocks.
        assert forall|i: int| self.num_data_blocks() as int <= i < self.index@.num_bits as int
            implies self.index@.is_bit_set(i) by {
            assert(i != block as int);
        };

        self.lemma_unset_bit_implies_free(bi);

        // Establish overflow bound for lemma_addr_injective.
        assert(self.data_addr as usize + self.num_data_blocks() * self.block_size <= usize::MAX) by {
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(
                (self.end_addr as usize - self.data_addr as usize) as int, self.block_size as int
            );
            vstd::arithmetic::mul::lemma_mul_is_commutative(
                self.num_data_blocks() as int, self.block_size as int
            );
        }
        Slab::lemma_addr_injective(
            self.data_addr, self.num_data_blocks(), self.block_size, block,
        );

        // allocated_addrs == old.remove(addr).
        assert forall|a: usize|
            #[trigger] self@.allocated_addrs.contains(a)
                || old_self@.allocated_addrs.remove(addr).contains(a)
            implies
            self@.allocated_addrs.contains(a)
                && old_self@.allocated_addrs.remove(addr).contains(a) by {
            if self@.allocated_addrs.contains(a) {
                let j = self.lemma_allocated_implies_set_bit(a);
                old_self.lemma_set_bit_implies_allocated(j);
            }
            if old_self@.allocated_addrs.remove(addr).contains(a) {
                let j = old_self.lemma_allocated_implies_set_bit(a);
                self.lemma_set_bit_implies_allocated(j);
            }
        }
        assert(self@.allocated_addrs =~= old_self@.allocated_addrs.remove(addr));

        // free_addrs == old.insert(addr).
        assert forall|a: usize|
            #[trigger] self@.free_addrs.contains(a)
                || old_self@.free_addrs.insert(addr).contains(a)
            implies
            self@.free_addrs.contains(a)
                && old_self@.free_addrs.insert(addr).contains(a) by {
            if self@.free_addrs.contains(a) {
                let j = self.lemma_free_implies_unset_bit(a);
                if j != bi { old_self.lemma_unset_bit_implies_free(j); }
            }
            if old_self@.free_addrs.insert(addr).contains(a) && a != addr {
                let j = old_self.lemma_free_implies_unset_bit(a);
                self.lemma_unset_bit_implies_free(j);
            }
        }
        assert(self@.free_addrs =~= old_self@.free_addrs.insert(addr));
    }
}

}
