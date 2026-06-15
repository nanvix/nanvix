    fn refcount(&self, frame: FrameAddress) -> Result<u8, Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();

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
