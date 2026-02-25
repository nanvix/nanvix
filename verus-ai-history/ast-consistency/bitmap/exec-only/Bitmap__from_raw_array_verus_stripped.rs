    pub fn from_raw_array(array: RawArray<u8>) -> Result<Self, Error>
    {
        // Check for overflow: array.len() * u8::BITS would overflow usize.
        // Verus note: checked_mul and closures are not supported in Verus,
        // so we use a manual overflow check. Semantically equivalent to
        // source's array.len().checked_mul(u8::BITS as usize).ok_or_else(...).
        if array.len() > usize::MAX / (u8::BITS as usize) {
            let reason: &str = "bitmap size overflow: array too large";
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        let number_of_bits: usize = array.len() * u8::BITS as usize;

        let result = Self {
            number_of_bits,
            bits: array,
            usage: 0,
        };
        Ok(result)
    }
