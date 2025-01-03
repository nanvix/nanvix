// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::{
    mem,
    x86::mem::paging::{
        flags::{
            AccessedFlag,
            DirtyFlag,
            PageCacheDisableFlag,
            PageWriteThroughFlag,
            PresentFlag,
            ReadWriteFlag,
            UserSupervisorFlag,
        },
        frame::FrameNumber,
    },
};

//==================================================================================================
// Page Directory Entry Flags
//==================================================================================================

///
/// # Description
///
/// A type that represents flags of a page directory entry.
///
#[derive(Debug)]
pub struct PageDirectoryEntryFlags {
    /// Present flag.
    present: PresentFlag,
    /// Read/write flag.
    read_write: ReadWriteFlag,
    /// User/supervisor flag.
    user_supervisor: UserSupervisorFlag,
    /// Page write-through flag.
    page_write_through: PageWriteThroughFlag,
    /// Page cache disable flag.
    page_cache_disable: PageCacheDisableFlag,
    /// Accessed flag.
    accessed: AccessedFlag,
    /// Dirty flag.
    dirty: DirtyFlag,
}

impl PageDirectoryEntryFlags {
    ///
    /// # Description
    ///
    /// Constructs a [`PageDirectoryEntryFlags`] with the given flags.
    ///
    /// # Parameters
    ///
    /// - `present`: The present flag.
    /// - `read_write`: The read/write flag.
    /// - `user_supervisor`: The user/supervisor flag.
    /// - `page_write_through`: The page write-through flag.
    /// - `page_cache_disable`: The page cache disable flag.
    /// - `accessed`: The accessed flag.
    /// - `dirty`: The dirty flag.
    ///
    /// # Returns
    ///
    /// A [`PageDirectoryEntryFlags`].
    ///
    pub fn new(
        present: PresentFlag,
        read_write: ReadWriteFlag,
        user_supervisor: UserSupervisorFlag,
        page_write_through: PageWriteThroughFlag,
        page_cache_disable: PageCacheDisableFlag,
        accessed: AccessedFlag,
        dirty: DirtyFlag,
    ) -> Self {
        Self {
            present,
            read_write,
            user_supervisor,
            page_write_through,
            page_cache_disable,
            accessed,
            dirty,
        }
    }

    ///
    /// # Description
    ///
    /// Constructs a [`PageDirectoryEntryFlags`] from a raw value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw value.
    ///
    /// # Returns
    ///
    /// A [`PageDirectoryEntryFlags`].
    ///
    fn from_raw_value(value: u32) -> Self {
        Self {
            present: PresentFlag::from_raw_value(value),
            read_write: ReadWriteFlag::from_raw_value(value),
            user_supervisor: UserSupervisorFlag::from_raw_value(value),
            page_write_through: PageWriteThroughFlag::from_raw_value(value),
            page_cache_disable: PageCacheDisableFlag::from_raw_value(value),
            accessed: AccessedFlag::from_raw_value(value),
            dirty: DirtyFlag::from_raw_value(value),
        }
    }

    ///
    /// # Description
    ///
    /// Converts a [`PageDirectoryEntryFlags`] into a raw value.
    ///
    /// # Returns
    ///
    /// The raw value.
    ///
    fn into_raw_value(self) -> u32 {
        let mut value: u32 = 0;

        value |= self.present.into_raw_value();
        value |= self.read_write.into_raw_value();
        value |= self.user_supervisor.into_raw_value();
        value |= self.page_write_through.into_raw_value();
        value |= self.page_cache_disable.into_raw_value();
        value |= self.accessed.into_raw_value();
        value |= self.dirty.into_raw_value();

        value
    }
}

//==================================================================================================
// Page Directory Entry
//==================================================================================================

///
/// # Description
///
/// A type that represents a page directory entry.
///
#[derive(Debug)]
pub struct PageDirectoryEntry {
    /// Flags.
    flags: PageDirectoryEntryFlags,
    /// Physical address of the page table.
    frame: FrameNumber,
}

impl PageDirectoryEntry {
    ///
    /// # Description
    ///
    /// Constructs a [`PageDirectoryEntry`] with the given flags and frame number.
    ///
    /// # Parameters
    ///
    /// - `flags`: The flags.
    /// - `frame`: The frame number.
    ///
    /// # Returns
    ///
    /// A [`PageDirectoryEntry`].
    ///
    pub fn new(flags: PageDirectoryEntryFlags, frame: FrameNumber) -> Self {
        Self { flags, frame }
    }

    ///
    /// # Description
    ///
    /// Constructs a [`PageDirectoryEntry`] from a raw value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw value.
    ///
    /// # Returns
    ///
    /// - `Some(`[`PageDirectoryEntry`]`)`: If the raw value is valid.
    /// - `None`: Otherwise.
    ///
    pub fn from_raw_value(value: u32) -> Option<Self> {
        Some(Self {
            flags: PageDirectoryEntryFlags::from_raw_value(value),
            frame: FrameNumber::from_raw_value(value as usize >> mem::FRAME_SHIFT)?,
        })
    }

    ///
    /// # Description
    ///
    /// Converts a [`PageDirectoryEntry`] into a raw value.
    ///
    /// # Returns
    ///
    /// The raw value.
    ///
    pub fn into_raw_value(self) -> u32 {
        let mut value: u32 = 0;

        value |= self.flags.into_raw_value();
        value |= (self.frame.into_raw_value() << mem::FRAME_SHIFT) as u32;

        value
    }

    ///
    /// # Description
    ///
    /// Checks if the target page directory entry is marked as present.
    ///
    /// # Returns
    ///
    /// `true`: If the target page directory entry is marked as present.
    /// `false`: Otherwise.
    ///
    pub fn is_present(&self) -> bool {
        match self.flags.present {
            PresentFlag::Present => true,
            PresentFlag::NotPresent => false,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the frame number of the target page directory entry.
    ///
    /// # Returns
    ///
    /// The frame address.
    ///
    pub fn frame(&self) -> FrameNumber {
        self.frame
    }
}
