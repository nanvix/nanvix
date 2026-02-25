    pub fn from_raw_array(array: RawArray<u8>) -> (result: Result<Self, Error>)
        requires
            array@.len() > 0,
            array@.len() * (u8::BITS as usize) < u32::MAX as usize,
            forall|i: int| 0 <= i < array@.len() ==> array@[i] == 0,
        ensures
            result is Ok ==> {
                let bitmap = result->Ok_0;
                &&& bitmap.inv()
                &&& bitmap@.number_of_bits() == array@.len() * (u8::BITS as int)
                &&& bitmap@.is_empty()
                &&& forall|i: int| 0 <= i < bitmap@.number_of_bits() ==> !bitmap.is_bit_set(i)
            },
            // Liveness: given preconditions, always succeeds.
            result is Ok,
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
        proof {
            result.lemma_zero_bytes_means_empty_set();
            Self::lemma_empty_set_finite();
        }
        Ok(result)
    }
