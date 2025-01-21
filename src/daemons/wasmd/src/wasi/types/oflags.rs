// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Open flags.
#[derive(Debug)]
pub struct OpenFlags {
    /// Create file if it does not exist.
    pub creat: bool,
    /// Fail if not a directory.
    pub directory: bool,
    /// Fail if file already exists.
    pub excl: bool,
    /// Truncate file to size zero.
    pub trunc: bool,
}

impl OpenFlags {
    const BIT_OFFSET_OF_CREAT: u32 = 0;
    const BIT_OFFSET_OF_DIRECTORY: u32 = 1;
    const BIT_OFFSET_OF_EXCL: u32 = 2;
    const BIT_OFFSET_OF_TRUNC: u32 = 3;
}

impl From<u32> for OpenFlags {
    fn from(val: u32) -> Self {
        Self {
            creat: val & (1 << Self::BIT_OFFSET_OF_CREAT) != 0,
            directory: val & (1 << Self::BIT_OFFSET_OF_DIRECTORY) != 0,
            excl: val & (1 << Self::BIT_OFFSET_OF_EXCL) != 0,
            trunc: val & (1 << Self::BIT_OFFSET_OF_TRUNC) != 0,
        }
    }
}

impl From<i32> for OpenFlags {
    fn from(val: i32) -> Self {
        Self::from(val as u32)
    }
}
