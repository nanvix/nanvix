    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        proof_decl! { let ghost old_self = *self; }
        let frame_number: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                proof! {
                    // `alloc` failed: the bitmap is full and its view is unchanged, so the whole
                    // allocator view is unchanged and there are no free frames.
                    lemma_view_determined(self, &old_self);
                    lemma_full_no_free(self);
                }
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?}");
                return Err(error);
            },
        };
        // Newly allocated frames have a single owner.
        #[cfg(not(verus_keep_ghost))]
        debug_assert_eq!(self.refcount[frame_number], 0);
        // The allocated index is in range and representable as a frame number; the proof below
        // discharges the conversions that follow.
        proof_decl! { let ghost pa: int = frame_number as int * spec_page_size(); }
        proof! {
            let idx = frame_number as int;
            assert(0 <= idx < old_self.bitmap@.num_bits);
            assert(!old_self.bitmap@.set_bits.contains(idx));
            assert(self.bitmap@.set_bits == old_self.bitmap@.set_bits.insert(idx));
            assert(self.bitmap@.num_bits == old_self.bitmap@.num_bits);
            // The freshly allocated slot was zero before this call.
            assert(old_self.refcount@[idx] == 0);
            // `pa` is the frame's base address.
            assert(pa == frame_addr_of(idx));
            vstd::arithmetic::div_mod::lemma_mod_multiples_basic(idx, spec_page_size());
            assert(pa % spec_page_size() == 0);
            assert(pa >= 0) by (nonlinear_arith) requires idx >= 0, spec_page_size() > 0, pa == idx * spec_page_size();
            lemma_frame_facts(&old_self, pa, idx);
            assert(old_self@.free_frames.contains(pa));
            // The index is a representable frame number: `internal_inv` carries
            // `num_bits <= spec_max() + 1`, and `idx < num_bits`, hence `idx <= spec_max()`.
            assert(idx < self.bitmap@.num_bits);
            assert(self.bitmap@.num_bits <= FrameNumber::spec_max() + 1);
            assert(idx <= FrameNumber::spec_max());
        }
        self.refcount[frame_number] = 1;
        proof! {
            let idx = frame_number as int;
            assert(self.refcount@ == old_self.refcount@.update(idx, 1));
            lemma_refcount_book(&old_self, self, idx, pa);
            lemma_internal_inv_implies_wf(self);
        }
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                proof! { assert(false); }
                let reason: &str = "frame number is out of bounds";
                #[cfg(not(verus_keep_ghost))]
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Attempt to convert the frame number to a frame address.
        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => {
                proof! {
                    assert(frame_address@ == pa);
                }
                Ok(frame_address)
            },
            Err(error) => {
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?}");
                Err(error)
            },
        }
    }
