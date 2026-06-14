// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::PteWord;
use vstd::prelude::*;

//==================================================================================================
// Enumerations
//==================================================================================================

///
/// # Description
///
/// A type that represents the present flag of a page table entry.
///
#[verus_verify]
#[derive(Clone, Copy, Debug)]
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
#[verus_verify]
#[derive(Clone, Copy, Debug)]
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
#[verus_verify]
#[derive(Clone, Copy, Debug)]
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
#[verus_verify]
#[derive(Clone, Copy, Debug)]
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
#[verus_verify]
#[derive(Clone, Copy, Debug)]
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
#[verus_verify]
#[derive(Clone, Copy, Debug)]
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
#[verus_verify]
#[derive(Clone, Copy, Debug)]
pub enum DirtyFlag {
    /// The page table entry has not been written.
    NotDirty = 0,
    /// The page table entry has been written.
    Dirty = (1 << Self::SHIFT),
}

///
/// # Description
///
/// A type that represents the page size flag of a page directory entry.
/// When set, the entry maps a large page (4 MiB on x86).
///
#[verus_verify]
#[derive(Clone, Copy, Debug)]
pub enum PageSizeFlag {
    /// Standard page size (4 KiB page table reference).
    Standard = 0,
    /// Large page (PS bit set).
    Large = (1 << Self::SHIFT),
}

///
/// # Description
///
/// A type that represents the copy-on-write flag of a page table entry. This is an
/// OS-defined flag that lives in one of the architecturally-available bits (AVL,
/// bit 9 of the x86 PTE). When set, the page is shared with another address space
/// and writes to it must trigger a copy on the page-fault path.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyOnWriteFlag {
    /// The page is not shared via copy-on-write.
    NotCopyOnWrite = 0,
    /// The page is shared via copy-on-write.
    CopyOnWrite = (1 << Self::SHIFT),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PresentFlag {
    /// Bit shift of the present flag in the page table entry.
    const SHIFT: PteWord = 0;
    /// Bit mask of the present flag in the page table entry.
    const MASK: PteWord = (1 << Self::SHIFT);

    pub fn from_raw_value(value: PteWord) -> Self {
        match value & Self::MASK {
            0 => PresentFlag::NotPresent,
            _ => PresentFlag::Present,
        }
    }

    pub fn into_raw_value(self) -> PteWord {
        self as PteWord
    }

    /// Checks if the present bit is set in a raw page table entry value.
    #[inline(always)]
    pub fn is_set(raw_entry: PteWord) -> bool {
        raw_entry & Self::MASK != 0
    }
}

impl ReadWriteFlag {
    /// Bit shift of the read/write flag in the page table entry.
    const SHIFT: PteWord = 1;
    /// Bit mask of the read/write flag in the page table entry.
    const MASK: PteWord = (1 << Self::SHIFT);

    pub fn from_raw_value(value: PteWord) -> Self {
        match value & Self::MASK {
            0 => ReadWriteFlag::ReadOnly,
            _ => ReadWriteFlag::ReadWrite,
        }
    }

    pub fn into_raw_value(self) -> PteWord {
        self as PteWord
    }
}

impl UserSupervisorFlag {
    /// Bit shift of the user/supervisor flag in the page table entry.
    const SHIFT: PteWord = 2;
    /// Bit mask of the user/supervisor flag in the page table entry.
    const MASK: PteWord = (1 << Self::SHIFT);

    pub fn from_raw_value(value: PteWord) -> Self {
        match value & Self::MASK {
            0 => UserSupervisorFlag::Supervisor,
            _ => UserSupervisorFlag::User,
        }
    }

    pub fn into_raw_value(self) -> PteWord {
        self as PteWord
    }
}

impl PageWriteThroughFlag {
    /// Bit shift of the page write-through flag in the page table entry.
    const SHIFT: PteWord = 3;
    /// Bit mask of the page write-through flag in the page table entry.
    const MASK: PteWord = (1 << Self::SHIFT);

    pub fn from_raw_value(value: PteWord) -> Self {
        match value & Self::MASK {
            0 => PageWriteThroughFlag::NotWriteThrough,
            _ => PageWriteThroughFlag::WriteThrough,
        }
    }

    pub fn into_raw_value(self) -> PteWord {
        self as PteWord
    }
}

impl PageCacheDisableFlag {
    /// Bit shift of the page cache disable flag in the page table entry.
    const SHIFT: PteWord = 4;
    /// Bit mask of the page cache disable flag in the page table entry.
    const MASK: PteWord = (1 << Self::SHIFT);

    pub fn from_raw_value(value: PteWord) -> Self {
        match value & Self::MASK {
            0 => PageCacheDisableFlag::CacheEnabled,
            _ => PageCacheDisableFlag::CacheDisabled,
        }
    }

    pub fn into_raw_value(self) -> PteWord {
        self as PteWord
    }
}

impl AccessedFlag {
    /// Bit shift of the accessed flag in the page table entry.
    const SHIFT: PteWord = 5;
    /// Bit mask of the accessed flag in the page table entry.
    const MASK: PteWord = (1 << Self::SHIFT);

    pub fn from_raw_value(value: PteWord) -> Self {
        match value & Self::MASK {
            0 => AccessedFlag::NotAccessed,
            _ => AccessedFlag::Accessed,
        }
    }

    pub fn into_raw_value(self) -> PteWord {
        self as PteWord
    }
}

impl DirtyFlag {
    /// Bit shift of the dirty flag in the page table entry.
    const SHIFT: PteWord = 6;
    /// Bit mask of the dirty flag in the page table entry.
    const MASK: PteWord = (1 << Self::SHIFT);

    pub fn from_raw_value(value: PteWord) -> Self {
        match value & Self::MASK {
            0 => DirtyFlag::NotDirty,
            _ => DirtyFlag::Dirty,
        }
    }

    pub fn into_raw_value(self) -> PteWord {
        self as PteWord
    }
}

impl PageSizeFlag {
    /// Bit shift of the page size flag in the page directory entry.
    const SHIFT: PteWord = 7;
    /// Bit mask of the page size flag in the page directory entry.
    const MASK: PteWord = (1 << Self::SHIFT);

    pub fn from_raw_value(value: PteWord) -> Self {
        match value & Self::MASK {
            0 => PageSizeFlag::Standard,
            _ => PageSizeFlag::Large,
        }
    }

    pub fn into_raw_value(self) -> PteWord {
        self as PteWord
    }
}

impl CopyOnWriteFlag {
    /// Bit shift of the copy-on-write flag in the page table entry. This lives in one of
    /// the OS-available (AVL) bits (9..=11) of the x86 PTE.
    const SHIFT: PteWord = 9;
    /// Bit mask of the copy-on-write flag in the page table entry.
    const MASK: PteWord = (1 << Self::SHIFT);

    pub fn from_raw_value(value: PteWord) -> Self {
        match value & Self::MASK {
            0 => CopyOnWriteFlag::NotCopyOnWrite,
            _ => CopyOnWriteFlag::CopyOnWrite,
        }
    }

    pub fn into_raw_value(self) -> PteWord {
        self as PteWord
    }
}
