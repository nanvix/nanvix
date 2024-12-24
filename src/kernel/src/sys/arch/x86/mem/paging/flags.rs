// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Enumerations
//==================================================================================================

///
/// # Description
///
/// A type that represents the present flag of a page table entry.
///
#[derive(Debug)]
pub enum PresentFlag {
    /// The page table entry is not present.
    NotPresent = 0,
    /// The page table entry is present.
    Present = (1 << Self::SHIFT),
}

///
/// # Description
///
/// A type that represents the read/write flag of a page table entry.
///
#[derive(Debug)]
pub enum ReadWriteFlag {
    /// The page table entry is read-only.
    ReadOnly = 0,
    /// The page table entry is read/write.
    ReadWrite = (1 << Self::SHIFT),
}

///
/// # Description
///
/// A type that represents the user/supervisor flag of a page table entry.
///
#[derive(Debug)]
pub enum UserSupervisorFlag {
    /// The page table entry is for supervisor mode.
    Supervisor = 0,
    /// The page table entry is for user mode.
    User = (1 << Self::SHIFT),
}

///
/// # Description
///
/// A type that represents the page write-through flag of a page table entry.
///
#[derive(Debug)]
pub enum PageWriteThroughFlag {
    /// The page table entry is not write-through.
    NotWriteThrough = 0,
    /// The page table entry is write-through.
    WriteThrough = (1 << Self::SHIFT),
}

///
/// # Description
///
/// A type that represents the page cache disable flag of a page table entry.
///
#[derive(Debug)]
pub enum PageCacheDisableFlag {
    /// The page table entry is not cache disabled.
    CacheEnabled = 0,
    /// The page table entry is cache disabled.
    CacheDisabled = (1 << Self::SHIFT),
}

///
/// # Description
///
/// A type that represents the accessed flag of a page table entry.
///
#[derive(Debug)]
pub enum AccessedFlag {
    /// The page table entry has not been accessed.
    NotAccessed = 0,
    /// The page table entry has been accessed.
    Accessed = (1 << Self::SHIFT),
}

///
/// # Description
///
/// A type that represents the dirty flag of a page table entry.
///
#[derive(Debug)]
pub enum DirtyFlag {
    /// The page table entry has not been written.
    NotDirty = 0,
    /// The page table entry has been written.
    Dirty = (1 << Self::SHIFT),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PresentFlag {
    /// Bit shift of the present flag in the page table entry.
    const SHIFT: u32 = 0;
    /// Bit mask of the present flag in the page table entry.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => PresentFlag::NotPresent,
            _ => PresentFlag::Present,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

impl ReadWriteFlag {
    /// Bit shift of the read/write flag in the page table entry.
    const SHIFT: u32 = 1;
    /// Bit mask of the read/write flag in the page table entry.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => ReadWriteFlag::ReadOnly,
            _ => ReadWriteFlag::ReadWrite,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

impl UserSupervisorFlag {
    /// Bit shift of the user/supervisor flag in the page table entry.
    const SHIFT: u32 = 2;
    /// Bit mask of the user/supervisor flag in the page table entry.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => UserSupervisorFlag::Supervisor,
            _ => UserSupervisorFlag::User,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

impl PageWriteThroughFlag {
    /// Bit shift of the page write-through flag in the page table entry.
    const SHIFT: u32 = 3;
    /// Bit mask of the page write-through flag in the page table entry.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => PageWriteThroughFlag::NotWriteThrough,
            _ => PageWriteThroughFlag::WriteThrough,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

impl PageCacheDisableFlag {
    /// Bit shift of the page cache disable flag in the page table entry.
    const SHIFT: u32 = 4;
    /// Bit mask of the page cache disable flag in the page table entry.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => PageCacheDisableFlag::CacheEnabled,
            _ => PageCacheDisableFlag::CacheDisabled,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

impl AccessedFlag {
    /// Bit shift of the accessed flag in the page table entry.
    const SHIFT: u32 = 5;
    /// Bit mask of the accessed flag in the page table entry.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => AccessedFlag::NotAccessed,
            _ => AccessedFlag::Accessed,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

impl DirtyFlag {
    /// Bit shift of the dirty flag in the page table entry.
    const SHIFT: u32 = 6;
    /// Bit mask of the dirty flag in the page table entry.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => DirtyFlag::NotDirty,
            _ => DirtyFlag::Dirty,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}
