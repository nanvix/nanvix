// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// File descriptor flags.
#[derive(Debug)]
pub struct FdFlags {
    /// Append mode:
    pub append: bool,
    /// Write according to synchronized I/O data integrity completion
    pub dsync: bool,
    /// Non-blocking mode.
    pub nonblock: bool,
    /// Synchronized read I/O operations.
    pub rsync: bool,
    /// Write according to synchronized I/O file integrity completion.
    pub sync: bool,
}

impl FdFlags {
    const BIT_OFFSET_OF_APPEND: u64 = 0;
    const BIT_OFFSET_OF_DSYNC: u64 = 1;
    const BIT_OFFSET_OF_NONBLOCK: u64 = 2;
    const BIT_OFFSET_OF_RSYNC: u64 = 3;
    const BIT_OFFSET_OF_SYNC: u64 = 4;
}

impl From<u64> for FdFlags {
    fn from(val: u64) -> Self {
        Self {
            append: val & (1 << Self::BIT_OFFSET_OF_APPEND) != 0,
            dsync: val & (1 << Self::BIT_OFFSET_OF_DSYNC) != 0,
            nonblock: val & (1 << Self::BIT_OFFSET_OF_NONBLOCK) != 0,
            rsync: val & (1 << Self::BIT_OFFSET_OF_RSYNC) != 0,
            sync: val & (1 << Self::BIT_OFFSET_OF_SYNC) != 0,
        }
    }
}

impl From<i64> for FdFlags {
    fn from(val: i64) -> Self {
        Self::from(val as u64)
    }
}

impl From<u32> for FdFlags {
    fn from(val: u32) -> Self {
        Self::from(val as u64)
    }
}

impl From<i32> for FdFlags {
    fn from(val: i32) -> Self {
        Self::from(val as u64)
    }
}
