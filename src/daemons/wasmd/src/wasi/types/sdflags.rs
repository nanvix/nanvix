// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::posix::sys::socket;

//==================================================================================================
// Structures
//==================================================================================================

/// Describes how a socket should be shutdown.
pub struct SdFlags {
    // Disables further receive operations.
    rd: bool,
    // Disables further send operations.
    wr: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SdFlags {
    const BIT_OFFSET_OF_RD: u64 = 0;
    const BIT_OFFSET_OF_WR: u64 = 1;
}

impl TryFrom<i32> for SdFlags {
    type Error = ();

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        // Check for invalid bits.
        if val & !((1 << Self::BIT_OFFSET_OF_RD) | (1 << Self::BIT_OFFSET_OF_WR)) != 0 {
            return Err(());
        }

        Ok(Self {
            rd: val & (1 << Self::BIT_OFFSET_OF_RD) != 0,
            wr: val & (1 << Self::BIT_OFFSET_OF_WR) != 0,
        })
    }
}

impl From<SdFlags> for socket::Shutdown {
    fn from(flags: SdFlags) -> Self {
        match (flags.rd, flags.wr) {
            (true, false) => socket::Shutdown::Read,
            (false, true) => socket::Shutdown::Write,
            (true, true) => socket::Shutdown::ReadWrite,
            _ => {
                // It is impossible to construct SdFlags with both rd and wr set to false.
                unreachable!("SdFlags with both rd and wr set to false");
            },
        }
    }
}
