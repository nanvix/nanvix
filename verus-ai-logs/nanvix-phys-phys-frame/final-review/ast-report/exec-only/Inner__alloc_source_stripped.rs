    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let frame_number: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?}");
                return Err(error);
            },
        };
        // Newly allocated frames have a single owner.
        debug_assert_eq!(self.refcount[frame_number], 0);
        self.refcount[frame_number] = 1;
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                let reason: &str = "frame number is out of bounds";
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Attempt to convert the frame number to a frame address.
        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => Ok(frame_address),
            Err(error) => {
                error!("{error:?}");
                Err(error)
            },
        }
    }
