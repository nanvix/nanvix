// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Lookup flags.
#[derive(Debug)]
pub struct LookupFlags {
    pub symlink_follow: bool,
}

impl LookupFlags {
    const BIT_OFFSET_OF_SYMLINK_FOLLOW: u32 = 0;
}

impl From<u32> for LookupFlags {
    fn from(val: u32) -> Self {
        Self {
            symlink_follow: val & (1 << Self::BIT_OFFSET_OF_SYMLINK_FOLLOW) != 0,
        }
    }
}

impl From<i32> for LookupFlags {
    fn from(val: i32) -> Self {
        Self::from(val as u32)
    }
}
