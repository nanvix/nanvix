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
// Page Table Entry Flags
//==================================================================================================

///
/// # Description
///
/// A type that represents flags of a page table entry.
///
#[derive(Debug)]
pub struct PageTableEntryFlags {
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

impl PageTableEntryFlags {
    ///
    /// # Description
    ///
    /// Constructs a [`PageTableEntryFlags`] with the given flags.
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
    /// A new [`PageTableEntryFlags`] with the given flags.
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
    /// Constructs a [`PageTableEntryFlags`] from a raw value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw value.
    ///
    /// # Returns
    ///
    /// A [`PageTableEntryFlags`].
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
    /// Converts a [`PageTableEntryFlags`] into a raw value.
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
// Page Table Entry
//==================================================================================================

///
/// # Description
///
/// A type that represents a page table entry.
///
#[derive(Debug)]
pub struct PageTableEntry {
    /// Flags.
    flags: PageTableEntryFlags,
    /// Physical address of the page frame.
    frame: FrameNumber,
}

impl PageTableEntry {
    ///
    /// # Description
    ///
    /// Constructs a [`PageTableEntry`] with the given flags and frame.
    ///
    /// # Parameters
    ///
    /// - `flags`: The flags.
    /// - `frame`: The frame number.
    ///
    /// # Returns
    ///
    /// A [`PageTableEntry`].
    ///
    pub fn new(flags: PageTableEntryFlags, frame: FrameNumber) -> Self {
        Self { flags, frame }
    }

    ///
    /// # Description
    ///
    /// Constructs a [`PageTableEntry`] from a raw value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw value.
    ///
    /// # Returns
    ///
    /// - `Some(`[`PageTableEntry`]`)`: If the raw value is valid.
    /// - `None`: Otherwise.
    ///
    pub fn from_raw_value(value: u32) -> Option<Self> {
        Some(Self {
            flags: PageTableEntryFlags::from_raw_value(value),
            frame: FrameNumber::from_raw_value(value as usize >> mem::FRAME_SHIFT)?,
        })
    }

    ///
    /// # Description
    ///
    /// Converts a [`PageTableEntry`] into a raw value.
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
    /// Returns the frame number associated with the target page table entry.
    ///
    /// # Returns
    ///
    /// The frame number associated with the target page table entry.
    ///
    pub fn frame_number(&self) -> FrameNumber {
        self.frame
    }

    ///
    /// # Description
    ///
    /// Checks if the target page table entry is marked as present.
    ///
    /// # Returns
    ///
    /// `true`: If the target page table entry is marked as present.
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
    /// Sets read/write flag in the target page.
    ///
    /// # Parameters
    ///
    /// - `read_write`: The read/write flag.
    ///
    pub fn set_read_write(&mut self, read_write: ReadWriteFlag) {
        self.flags.read_write = read_write;
    }

    ///
    /// # Description
    ///
    /// Sets user/supervisor flag in the target page.
    ///
    /// # Parameters
    ///
    /// - `user_supervisor`: The user/supervisor flag.
    ///
    pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag) {
        self.flags.user_supervisor = user_supervisor;
    }
}
