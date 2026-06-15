    fn share(&mut self, frame: FrameAddress) -> Result<(), Error> {
        proof_decl! {
            let ghost pa: int = frame@;
            let ghost old_self = *self;
        }
        let raw: usize = frame.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        proof! {
            assert(mem::FRAME_SIZE as int == spec_page_size());
            assert(raw as int == pa);
            assert(pa % spec_page_size() == 0);
            assert(frame_number as int == pa / spec_page_size());
            lemma_frame_facts(self, pa, frame_number as int);
        }

        if frame_number >= self.refcount.len() {
            proof! {
                assert(self.refcount@.len() >= self.bitmap@.num_bits);
            }
            let reason: &str = "frame number out of bounds";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // The frame must currently have at least one owner. Sharing an unallocated
        // frame is a logic error.
        if self.refcount[frame_number] == 0 {
            proof! {
                let fnn = frame_number as int;
                if fnn < self.bitmap@.num_bits {
                    assert(self.bitmap@.set_bits.contains(fnn) <==> self.refcount@[fnn] > 0);
                }
                assert(!self.bitmap@.set_bits.contains(fnn));
            }
            let reason: &str = "cannot share an unallocated frame";
            #[cfg(not(verus_keep_ghost))]
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        let new_count: u8 = match self.refcount[frame_number].checked_add(1) {
            Some(n) => n,
            None => {
                proof! {
                    // The slot is saturated at 255, so the frame is allocated with refcount 255.
                    let fnn = frame_number as int;
                    assert(self.refcount@[fnn] == 255);
                    assert(fnn < self.bitmap@.num_bits);
                    assert(self.bitmap@.set_bits.contains(fnn));
                    assert(self@.refcounts[pa] == self.refcount@[fnn]);
                }
                let reason: &str = "frame reference count overflow";
                #[cfg(not(verus_keep_ghost))]
                error!("{reason} (frame={frame:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };
        self.refcount[frame_number] = new_count;

        proof! {
            let fnn = frame_number as int;
            // The frame is allocated: its slot was non-zero, so its index is bitmap-managed and set.
            assert(old_self.refcount@[fnn] > 0);
            assert(fnn < old_self.bitmap@.num_bits);
            assert(old_self.bitmap@.set_bits.contains(fnn));
            assert(self.bitmap == old_self.bitmap);
            assert(self.refcount@ == old_self.refcount@.update(fnn, new_count));
            assert(new_count as int == old_self.refcount@[fnn] + 1);
            // The bump keeps the bit set and the slot positive, preserving `internal_inv` and
            // changing the view only by incrementing `pa`'s refcount.
            lemma_refcount_bump(&old_self, self, fnn, new_count, pa);
            lemma_internal_inv_implies_wf(self);
            assert(new_count as int == old_self@.refcounts[pa] + 1);
        }
        Ok(())
    }
