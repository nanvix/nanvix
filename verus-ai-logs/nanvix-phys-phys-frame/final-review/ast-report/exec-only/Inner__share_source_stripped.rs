    fn share(&mut self, frame: FrameAddress) -> Result<(), Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();

        if frame_number >= self.refcount.len() {
            let reason: &str = "frame number out of bounds";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        // The frame must currently have at least one owner. Sharing an unallocated
        // frame is a logic error.
        if self.refcount[frame_number] == 0 {
            let reason: &str = "cannot share an unallocated frame";
            error!("{reason} (frame={frame:?})");
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }

        self.refcount[frame_number] = match self.refcount[frame_number].checked_add(1) {
            Some(n) => n,
            None => {
                let reason: &str = "frame reference count overflow";
                error!("{reason} (frame={frame:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        Ok(())
    }
