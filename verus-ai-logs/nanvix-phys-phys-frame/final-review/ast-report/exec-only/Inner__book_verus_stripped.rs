    fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        
        let raw: usize = phys_addr.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        
        match self.bitmap.set(frame_number) {
            Ok(()) => {
                
                debug_assert_eq!(self.refcount[frame_number], 0);
                
                self.refcount[frame_number] = 1;
                
                Ok(())
            },
            Err(error) => {
                
                
                error!("{error:?} (phys_addr={phys_addr:?})");
                Err(error)
            },
        }
    }
