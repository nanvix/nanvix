// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pal::fs::FileWhence;
use ::num_enum::TryFromPrimitive;

//==================================================================================================
// Enumerations
//==================================================================================================

/// Used for representing the position relative to which to set the offset of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum Whence {
    /// The offset is set to `offset`.
    Set = 0,
    /// The offset is set to its current location plus `offset`.
    Cur = 1,
    /// The offset is set to the end of the file plus `offset`.
    End = 2,
}
::nvx::sys::static_assert_alignment!(Whence, 1);
::nvx::sys::static_assert_size!(Whence, 1);

//==================================================================================================
// Implementations
//==================================================================================================

impl TryFrom<i32> for Whence {
    type Error = ();

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        u8::try_from(val)
            .ok()
            .and_then(|v| Whence::try_from(v).ok())
            .ok_or(())
    }
}

impl From<FileWhence> for Whence {
    fn from(whence: FileWhence) -> Self {
        match whence {
            FileWhence::Set => Whence::Set,
            FileWhence::Cur => Whence::Cur,
            FileWhence::End => Whence::End,
        }
    }
}

impl From<Whence> for FileWhence {
    fn from(whence: Whence) -> Self {
        match whence {
            Whence::Set => FileWhence::Set,
            Whence::Cur => FileWhence::Cur,
            Whence::End => FileWhence::End,
        }
    }
}
