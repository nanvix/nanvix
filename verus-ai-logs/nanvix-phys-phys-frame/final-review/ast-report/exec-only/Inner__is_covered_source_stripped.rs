    fn is_covered(&self, phys_addr: PageAligned<PhysicalAddress>) -> bool {
        let frame_number: usize = phys_addr.into_frame_number().into_raw_value();
        frame_number < self.bitmap.number_of_bits()
    }
