    fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error> {
        proof_decl! {
            let ghost old_self = *self;
            let ghost ps = spec_page_size();
        }
        let start_raw: usize = region.start().into_raw_value();
        let size: usize = region.size();
        // Compute the frame index by division (rather than `into_frame_number`), matching the
        // rest of this module: division needs no representable-frame-number precondition.
        let start_frame_number: usize = start_raw / mem::FRAME_SIZE;
        let count: usize = size / mem::FRAME_SIZE;

        proof! {
            assert(mem::FRAME_SIZE as int == ps);
            assert(start_raw as int == region@.start);
            assert(size as int == region@.size);
            assert(region@.start % ps == 0);
            assert(region@.size % ps == 0);
            assert(region@.size > 0);
            assert(start_frame_number as int == region@.start / ps);
            assert(count as int == region@.size / ps);
            // `region` is page-aligned and non-empty, so the booked range is non-empty.
            assert(count as int >= 1) by (nonlinear_arith)
                requires
                    ps > 0,
                    region@.size > 0,
                    region@.size % ps == 0,
                    count as int == region@.size / ps;
        }
        proof_decl! { let ghost lo: int = start_frame_number as int; }
        proof_decl! { let ghost hi: int = lo + count as int; }
        proof! {
            // `hi == (region@.start + region@.size) / ps` because both endpoints are page-aligned.
            assert(region@.start == ps * lo) by (nonlinear_arith)
                requires
                    ps > 0,
                    region@.start % ps == 0,
                    lo == region@.start / ps;
            assert(region@.size == ps * (count as int)) by (nonlinear_arith)
                requires
                    ps > 0,
                    region@.size % ps == 0,
                    count as int == region@.size / ps;
            assert(region@.start + region@.size == ps * hi) by (nonlinear_arith)
                requires
                    region@.start == ps * lo,
                    region@.size == ps * (count as int),
                    hi == lo + count as int;
            assert((region@.start + region@.size) / ps == hi) by (nonlinear_arith)
                requires ps > 0, region@.start + region@.size == ps * hi;
        }

        let nbits: usize = self.bitmap.number_of_bits();
        // Reject ranges that fall outside the bitmap up front. Some frame in the range would be
        // uncovered, hence not free, so the request must fail with the state left untouched.
        if start_frame_number >= nbits || count > nbits - start_frame_number {
            proof! {
                lemma_view_determined(self, &old_self);
                // Exhibit an in-range frame number `k` that the bitmap does not cover.
                let k: int = if start_frame_number >= nbits { lo } else { nbits as int };
                assert(lo <= k < hi);
                assert(k >= self.bitmap@.num_bits);
                let addr: int = frame_addr_of(k);
                assert(frame_set(lo, hi).contains(addr));
                assert(!old_self@.free_frames.contains(addr)) by {
                    if old_self@.free_frames.contains(addr) {
                        let i = choose|i: int|
                            0 <= i < old_self.bitmap@.num_bits
                                && !(#[trigger] old_self.bitmap@.set_bits.contains(i))
                                && addr == frame_addr_of(i);
                        assert(addr == i * ps);
                        assert(addr == k * ps);
                        assert(i == k) by (nonlinear_arith)
                            requires ps > 0, i * ps == k * ps;
                        assert(false);
                    }
                }
                lemma_map_range_is_frame_set(lo, hi);
                lemma_region_frames_eq(region@.start, region@.size, lo, hi);
                assert(region@.start / spec_page_size() == lo);
                assert((region@.start + region@.size) / spec_page_size() == hi);
                let frames = vstd::set_lib::set_int_range(
                    region@.start / spec_page_size(),
                    (region@.start + region@.size) / spec_page_size())
                    .map(|i: int| i * spec_page_size());
                assert(frames =~= frame_set(lo, hi));
                assert(frames.contains(addr));
                assert(!frames.subset_of(old_self@.free_frames));
            }
            let reason: &str = "frame index not covered by the bitmap";
            #[cfg(not(verus_keep_ghost))]
            error!("{} (region={:?})", reason, region);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        let end_frame_number: usize = start_frame_number + count;
        proof! {
            assert(hi == end_frame_number as int);
            assert(hi <= self.bitmap@.num_bits);
        }

        // Check that every frame in the range is currently free. An already-allocated frame
        // indicates a memory layout bug, so the whole request fails without mutating the state.
        #[cfg_attr(verus_keep_ghost, verus_spec(
            invariant
                lo == start_frame_number as int,
                hi == end_frame_number as int,
                region@.start / spec_page_size() == lo,
                (region@.start + region@.size) / spec_page_size() == hi,
                lo <= index <= hi,
                self.inv(),
                self.bitmap@.num_bits == old_self.bitmap@.num_bits,
                self.bitmap@.set_bits == old_self.bitmap@.set_bits,
                self.refcount@ == old_self.refcount@,
                hi <= self.bitmap@.num_bits,
                forall|j: int| lo <= j < index ==> !self.bitmap@.set_bits.contains(j),
            decreases hi - index,
        ))]
        for index in start_frame_number..end_frame_number {
            match self.bitmap.test(index) {
                Ok(false) => {},
                Ok(true) => {
                    proof! {
                        lemma_view_determined(self, &old_self);
                        let k: int = index as int;
                        assert(lo <= k < hi);
                        assert(self.bitmap@.set_bits.contains(k));
                        let addr: int = frame_addr_of(k);
                        assert(frame_set(lo, hi).contains(addr));
                        assert(!old_self@.free_frames.contains(addr)) by {
                            if old_self@.free_frames.contains(addr) {
                                let i = choose|i: int|
                                    0 <= i < old_self.bitmap@.num_bits
                                        && !(#[trigger] old_self.bitmap@.set_bits.contains(i))
                                        && addr == frame_addr_of(i);
                                assert(addr == i * ps);
                                assert(addr == k * ps);
                                assert(i == k) by (nonlinear_arith)
                                    requires ps > 0, i * ps == k * ps;
                                assert(false);
                            }
                        }
                        lemma_map_range_is_frame_set(lo, hi);
                        lemma_region_frames_eq(region@.start, region@.size, lo, hi);
                        assert(region@.start / spec_page_size() == lo);
                        assert((region@.start + region@.size) / spec_page_size() == hi);
                        let frames = vstd::set_lib::set_int_range(
                            region@.start / spec_page_size(),
                            (region@.start + region@.size) / spec_page_size())
                            .map(|i: int| i * spec_page_size());
                        assert(frames =~= frame_set(lo, hi));
                        assert(frames.contains(addr));
                        assert(!frames.subset_of(old_self@.free_frames));
                    }
                    let conflicting_addr: usize = index * mem::FRAME_SIZE;
                    let region_start: usize = region.start().into_raw_value();
                    let region_end: usize = region_start.saturating_add(region.size());
                    let reason: &str = "frame is already allocated";
                    #[cfg(not(verus_keep_ghost))]
                    error!(
                        "{} (frame={:#010x}, region_start={:#010x}, region_end={:#010x})",
                        reason, conflicting_addr, region_start, region_end
                    );
                    return Err(Error::new(ErrorCode::OutOfMemory, reason));
                },
                Err(err) => {
                    proof! {
                        // `test` only fails when the index is out of range, but coverage above
                        // guarantees `index < num_bits`; this branch is unreachable.
                        assert(index < self.bitmap@.num_bits);
                        assert(false);
                    }
                    return Err(err);
                },
            }
        }
        proof! {
            // Coverage succeeded: the whole range was free in `old_self`.
            assert(self.bitmap@.set_bits == old_self.bitmap@.set_bits);
            assert forall|j: int| lo <= j < hi implies
                !old_self.bitmap@.set_bits.contains(j) by {}
        }

        // Book every frame in the range.
        #[cfg_attr(verus_keep_ghost, verus_spec(
            invariant
                lo == start_frame_number as int,
                hi == end_frame_number as int,
                region@.start / spec_page_size() == lo,
                (region@.start + region@.size) / spec_page_size() == hi,
                lo <= index <= hi,
                self.bitmap.inv(),
                self.bitmap@.num_bits == old_self.bitmap@.num_bits,
                self.refcount@.len() == old_self.refcount@.len(),
                self.refcount@.len() >= self.bitmap@.num_bits,
                hi <= self.bitmap@.num_bits,
                self.bitmap@.set_bits == old_self.bitmap@.set_bits.union(
                    ::bitmap::BitmapView::range_set(lo, index as int)),
                forall|j: int| lo <= j < hi ==> !old_self.bitmap@.set_bits.contains(j),
                forall|j: int| lo <= j < index ==> self.refcount@[j] == 1,
                forall|k: int| 0 <= k < self.refcount@.len() && !(lo <= k < index)
                    ==> self.refcount@[k] == old_self.refcount@[k],
            decreases hi - index,
        ))]
        for index in start_frame_number..end_frame_number {
            proof! {
                // The current bit is unset, so `set` will succeed.
                assert(!::bitmap::BitmapView::range_set(lo, index as int).contains(index as int));
                assert(!self.bitmap@.set_bits.contains(index as int));
                assert(index < self.bitmap@.num_bits);
            }
            proof_decl! { let ghost pre_set = *self; }
            if let Err(error) = self.bitmap.set(index) {
                proof! {
                    // `set` only fails when the bit is out of range or already set, both excluded.
                    assert(false);
                }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
            proof! {
                // `set` inserted `index`; combined with the prefix this extends the booked range.
                assert(self.bitmap@.set_bits == pre_set.bitmap@.set_bits.insert(index as int));
                assert(self.bitmap@.set_bits
                    == old_self.bitmap@.set_bits.union(
                        ::bitmap::BitmapView::range_set(lo, index as int + 1))) by {
                    assert(::bitmap::BitmapView::range_set(lo, index as int).insert(index as int)
                        =~= ::bitmap::BitmapView::range_set(lo, index as int + 1));
                }
            }
            #[cfg(not(verus_keep_ghost))]
            debug_assert_eq!(self.refcount[index], 0);
            proof_decl! { let ghost pre_ref = *self; }
            self.refcount[index] = 1;
            proof! {
                assert(self.refcount@ == pre_ref.refcount@.update(index as int, 1));
                assert(self.bitmap == pre_ref.bitmap);
            }
        }

        proof! {
            assert(self.bitmap@.set_bits == old_self.bitmap@.set_bits.union(
                ::bitmap::BitmapView::range_set(lo, hi)));
            assert forall|j: int| lo <= j < hi implies self.refcount@[j] == 1 by {}
            assert forall|k: int| 0 <= k < self.refcount@.len() && !(lo <= k < hi) implies
                self.refcount@[k] == old_self.refcount@[k] by {}
            assert(region@.start / spec_page_size() == lo);
            assert((region@.start + region@.size) / spec_page_size() == hi);
            // Establish `internal_inv` and the contract's `frames`-based view transition.
            lemma_alloc_range_view(&old_self, self, region@.start, region@.size, lo, hi);
            lemma_internal_inv_implies_wf(self);
            let frames = vstd::set_lib::set_int_range(
                region@.start / spec_page_size(),
                (region@.start + region@.size) / spec_page_size())
                .map(|i: int| i * spec_page_size());
            assert(self@ == FrameAllocView {
                allocated_frames: old_self@.allocated_frames.union(frames),
                free_frames: old_self@.free_frames.difference(frames),
                refcounts: old_self@.refcounts.union_prefer_right(
                    Map::new(|addr: int| frames.contains(addr), |addr: int| 1int)),
            });
        }
        Ok(())
    }
