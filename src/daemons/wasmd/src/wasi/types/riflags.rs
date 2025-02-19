// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

// Flags provided to sock_recv.
#[derive(Debug)]
pub struct RiFlags {
    /// Read data into the buffer without consuming it.
    pub peek: bool,
    /// Receive out-of-band data.
    pub oob: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RiFlags {
    const BIT_OFFSET_OF_PEEK: u32 = 0;
    const BIT_OFFSET_OF_OOB: u32 = 1;
}

impl TryFrom<i32> for RiFlags {
    type Error = ();

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        // Check for invalid bits.
        if val & !((1 << Self::BIT_OFFSET_OF_PEEK) | (1 << Self::BIT_OFFSET_OF_OOB)) != 0 {
            return Err(());
        }

        Ok(Self {
            peek: val & (1 << Self::BIT_OFFSET_OF_PEEK) != 0,
            oob: val & (1 << Self::BIT_OFFSET_OF_OOB) != 0,
        })
    }
}
