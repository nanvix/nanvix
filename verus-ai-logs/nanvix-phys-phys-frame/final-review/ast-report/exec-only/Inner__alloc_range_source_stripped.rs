    fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error> {
        let start_frame_number: usize = region.start().into_frame_number().into_raw_value();
        let end_frame_number: usize = start_frame_number + region.size() / mem::FRAME_SIZE - 1;

        // Check that all frames in the range are covered by the bitmap and free,
        // then book them. Uncovered frames indicate a memory layout bug.
        //
        // The coverage check runs unconditionally — including optimized builds —
        // because out-of-bounds indices must be rejected before attempting to set them.
        // This loop runs only at boot when booking memory regions, so the overhead is negligible.
        for index in start_frame_number..=end_frame_number {
            if index >= self.bitmap.number_of_bits() {
                let uncovered_addr: usize = index * mem::FRAME_SIZE;
                let reason: &str = "frame index not covered by the bitmap";
                error!("{} (frame={:#010x}, region={:?})", reason, uncovered_addr, region);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
            match self.bitmap.test(index) {
                Ok(false) => {},
                Ok(true) => {
                    let conflicting_addr: usize = index * mem::FRAME_SIZE;
                    let region_start: usize = region.start().into_raw_value();
                    let region_end: usize = region_start.saturating_add(region.size());
                    let reason: &str = "frame is already allocated";
                    error!(
                        "{} (frame={:#010x}, region_start={:#010x}, region_end={:#010x})",
                        reason, conflicting_addr, region_start, region_end
                    );
                    return Err(Error::new(ErrorCode::OutOfMemory, reason));
                },
                Err(err) => return Err(err),
            }
        }

        // Book all frames in the range.
        for index in start_frame_number..=end_frame_number {
            if let Err(error) = self.bitmap.set(index) {
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
            debug_assert_eq!(self.refcount[index], 0);
            self.refcount[index] = 1;
        }

        Ok(())
    }
