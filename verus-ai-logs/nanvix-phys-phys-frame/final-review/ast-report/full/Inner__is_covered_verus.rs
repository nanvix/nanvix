    fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool {
        proof_decl! {
            let ghost pa: int = phys_addr@;
        }
        // Compute the frame index by division instead of `into_frame_number()`. Both yield
        // `phys_addr@ / FRAME_SIZE`, but division needs no representable-frame-number precondition
        // and cannot panic on the (reserved) top-of-memory frame.
        let raw: usize = phys_addr.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        let nbits: usize = self.bitmap.number_of_bits();
        proof! {
            assert(mem::FRAME_SIZE as int == spec_page_size());
            assert(raw as int == pa);
            assert(pa % spec_page_size() == 0);
            assert(frame_number as int == pa / spec_page_size());
            lemma_frame_facts(self, pa, frame_number as int);
        }
        frame_number < nbits
    }
