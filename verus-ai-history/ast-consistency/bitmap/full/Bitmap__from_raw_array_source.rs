    pub fn from_raw_array(array: RawArray<u8>) -> Result<Self, Error> {
        let number_of_bits: usize =
            array.len().checked_mul(u8::BITS as usize).ok_or_else(|| {
                Error::new(ErrorCode::InvalidArgument, "bitmap size overflow: array too large")
            })?;

        // Note: RawArray guarantees zero-initialization of the backing storage.

        Ok(Self {
            number_of_bits,
            bits: array,
            usage: 0,
        })
    }
