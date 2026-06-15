    fn refcount(&self, frame: FrameAddress) -> Result<u8, Error> {
        
        
        let raw: usize = frame.into_raw_value();
        let frame_number: usize = raw / mem::FRAME_SIZE;
        

        if frame_number >= self.refcount.len() {
            
            let reason: &str = "frame number out of bounds";
            
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        if self.refcount[frame_number] == 0 {
            
            let reason: &str = "frame is not allocated";
            
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        
        Ok(self.refcount[frame_number])
    }
