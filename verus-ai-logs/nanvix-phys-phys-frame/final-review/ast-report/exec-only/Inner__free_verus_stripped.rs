    fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {
        
        let raw: usize = frame.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        

        if frame_number >= self.refcount.len() {
            
            let reason: &str = "frame number out of bounds";
            
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // Reject double-frees: the frame must currently have at least one owner.
        if self.refcount[frame_number] == 0 {
            
            let reason: &str = "frame is already free";
            
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        

        self.refcount[frame_number] -= 1;

        // Only release the bit in the bitmap when the last owner releases the frame.
        if self.refcount[frame_number] == 0 {
            
            match self.bitmap.clear(frame_number) {
                Ok(()) => {
                    
                    Ok(())
                },
                Err(error) => {
                    
                    
                    error!("{error:?} (frame={frame:?})");
                    Err(error)
                },
            }
        } else {
            
            Ok(())
        }
    }
