    fn alloc_contiguous(&mut self, count: usize) -> Result<FrameAddress, Error> {
        let frame_number: usize = match self.bitmap.alloc_range(count) {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?} (count={count})");
                return Err(error);
            },
        };
        // Newly allocated frames have a single owner.
        for i in frame_number..frame_number + count {
            debug_assert_eq!(self.refcount[i], 0);
            self.refcount[i] = 1;
        }
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                let reason: &str = "frame number is out of bounds";
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => Ok(frame_address),
            Err(error) => {
                error!("{error:?}");
                Err(error)
            },
        }
    }
