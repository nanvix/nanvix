// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

// Flags provided to sock_recv.
#[derive(Debug)]
pub struct SiFlags {
    /// Always zero.
    pub zero: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl TryFrom<i32> for SiFlags {
    type Error = ();

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(Self { zero: false }),
            _ => Err(()),
        }
    }
}
