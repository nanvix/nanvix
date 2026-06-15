    fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error> {
        
        let start_raw: usize = region.start().into_raw_value();
        let size: usize = region.size();
        // Compute the frame index by division (rather than `into_frame_number`), matching the
        // rest of this module: division needs no representable-frame-number precondition.
        let start_frame_number: usize = start_raw / mem::FRAME_SIZE;
        let count: usize = size / mem::FRAME_SIZE;

        
        
        
        

        let nbits: usize = self.bitmap.number_of_bits();
        // Reject ranges that fall outside the bitmap up front. Some frame in the range would be
        // uncovered, hence not free, so the request must fail with the state left untouched.
        if start_frame_number >= nbits || count > nbits - start_frame_number {
            
            let reason: &str = "frame index not covered by the bitmap";
            
            error!("{} (region={:?})", reason, region);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        let end_frame_number: usize = start_frame_number + count;
        

        // Check that every frame in the range is currently free. An already-allocated frame
        // indicates a memory layout bug, so the whole request fails without mutating the state.
        
        for index in start_frame_number..end_frame_number {
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
                Err(err) => {
                    
                    return Err(err);
                },
            }
        }
        

        // Book every frame in the range.
        
        for index in start_frame_number..end_frame_number {
            
            
            if let Err(error) = self.bitmap.set(index) {
                
                
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
            
            
            debug_assert_eq!(self.refcount[index], 0);
            
            self.refcount[index] = 1;
            
        }

        
        Ok(())
    }
