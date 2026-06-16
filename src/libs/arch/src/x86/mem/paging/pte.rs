// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    mem::{
        self,
        paging::TableEntry,
    },
    x86::mem::paging::{
        flags::{
            AccessedFlag,
            CopyOnWriteFlag,
            DirtyFlag,
            PageCacheDisableFlag,
            PageWriteThroughFlag,
            PresentFlag,
            ReadWriteFlag,
            UserSupervisorFlag,
        },
        frame::FrameNumber,
        PteWord,
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
#[derive(Clone, Copy, Debug)]
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
    /// Copy-on-write flag (OS-defined, stored in an AVL bit).
    cow: CopyOnWriteFlag,
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
            cow: CopyOnWriteFlag::NotCopyOnWrite,
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
    pub(crate) fn from_raw_value(value: PteWord) -> Self {
        Self {
            present: PresentFlag::from_raw_value(value),
            read_write: ReadWriteFlag::from_raw_value(value),
            user_supervisor: UserSupervisorFlag::from_raw_value(value),
            page_write_through: PageWriteThroughFlag::from_raw_value(value),
            page_cache_disable: PageCacheDisableFlag::from_raw_value(value),
            accessed: AccessedFlag::from_raw_value(value),
            dirty: DirtyFlag::from_raw_value(value),
            cow: CopyOnWriteFlag::from_raw_value(value),
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
    pub(crate) fn into_raw_value(self) -> PteWord {
        let mut value: PteWord = 0;

        value |= self.present.into_raw_value();
        value |= self.read_write.into_raw_value();
        value |= self.user_supervisor.into_raw_value();
        value |= self.page_write_through.into_raw_value();
        value |= self.page_cache_disable.into_raw_value();
        value |= self.accessed.into_raw_value();
        value |= self.dirty.into_raw_value();
        value |= self.cow.into_raw_value();

        value
    }

    ///
    /// # Description
    ///
    /// Checks if the present flag is set.
    ///
    /// # Returns
    ///
    /// `true` if the present flag is set, `false` otherwise.
    ///
    #[inline(always)]
    pub fn is_present(&self) -> bool {
        matches!(self.present, PresentFlag::Present)
    }

    ///
    /// # Description
    ///
    /// Checks if the user flag is set (i.e., user-mode access is allowed).
    ///
    /// # Returns
    ///
    /// `true` if the user flag is set, `false` otherwise.
    ///
    #[inline(always)]
    pub fn is_user(&self) -> bool {
        matches!(self.user_supervisor, UserSupervisorFlag::User)
    }

    ///
    /// # Description
    ///
    /// Checks if the read/write flag is set (i.e., the page is writable).
    ///
    /// # Returns
    ///
    /// `true` if the page is writable, `false` otherwise.
    ///
    #[inline(always)]
    pub fn is_writable(&self) -> bool {
        matches!(self.read_write, ReadWriteFlag::ReadWrite)
    }

    ///
    /// # Description
    ///
    /// Sets read/write flag.
    ///
    /// # Parameters
    ///
    /// - `read_write`: The read/write flag.
    ///
    #[inline(always)]
    pub fn set_read_write(&mut self, read_write: ReadWriteFlag) {
        self.read_write = read_write;
    }

    ///
    /// # Description
    ///
    /// Sets user/supervisor flag.
    ///
    /// # Parameters
    ///
    /// - `user_supervisor`: The user/supervisor flag.
    ///
    #[inline(always)]
    pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag) {
        self.user_supervisor = user_supervisor;
    }

    ///
    /// # Description
    ///
    /// Checks if the copy-on-write flag is set.
    ///
    /// # Returns
    ///
    /// `true` if the copy-on-write flag is set, `false` otherwise.
    ///
    #[inline(always)]
    pub fn is_cow(&self) -> bool {
        matches!(self.cow, CopyOnWriteFlag::CopyOnWrite)
    }

    ///
    /// # Description
    ///
    /// Sets the copy-on-write flag.
    ///
    /// # Parameters
    ///
    /// - `cow`: The copy-on-write flag.
    ///
    #[inline(always)]
    pub fn set_cow(&mut self, cow: CopyOnWriteFlag) {
        self.cow = cow;
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
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry {
    /// Flags.
    flags: PageTableEntryFlags,
    /// Physical address of the page frame.
    frame: FrameNumber,
}

impl PageTableEntry {
    /// Size in bytes of the hardware page table entry representation.
    pub const SIZE: usize = ::core::mem::size_of::<PteWord>();

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
    pub fn from_raw_value(value: PteWord) -> Option<Self> {
        use crate::x86::mem::paging::PHYS_ADDR_MASK;
        Some(Self {
            flags: PageTableEntryFlags::from_raw_value(value),
            frame: FrameNumber::from_raw_value(
                (value & PHYS_ADDR_MASK) as usize >> mem::FRAME_SHIFT,
            )?,
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
    pub fn into_raw_value(self) -> PteWord {
        let mut value: PteWord = 0;

        value |= self.flags.into_raw_value();
        value |= (self.frame.into_raw_value() << crate::mem::FRAME_SHIFT) as PteWord;

        value
    }

    ///
    /// # Description
    ///
    /// Returns the flags associated with the target page table entry.
    ///
    /// # Returns
    ///
    /// The flags.
    ///
    pub fn flags(&self) -> PageTableEntryFlags {
        self.flags
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
    /// Returns the physical address (frame number × frame size) of the page frame.
    ///
    /// # Returns
    ///
    /// The physical address.
    ///
    pub fn frame_address(&self) -> usize {
        self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
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
        self.flags.is_present()
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
        self.flags.set_read_write(read_write);
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
        self.flags.set_user_supervisor(user_supervisor);
    }

    ///
    /// # Description
    ///
    /// Checks whether the target page table entry is marked copy-on-write.
    ///
    /// # Returns
    ///
    /// `true` if the entry is marked copy-on-write, `false` otherwise.
    ///
    pub fn is_cow(&self) -> bool {
        self.flags.is_cow()
    }

    ///
    /// # Description
    ///
    /// Sets the copy-on-write flag in the target page table entry.
    ///
    /// # Parameters
    ///
    /// - `cow`: The copy-on-write flag.
    ///
    pub fn set_cow(&mut self, cow: CopyOnWriteFlag) {
        self.flags.set_cow(cow);
    }
}

impl TableEntry for PageTableEntry {
    fn from_raw(raw: PteWord) -> Option<Self> {
        Self::from_raw_value(raw)
    }

    fn raw(self) -> PteWord {
        self.into_raw_value()
    }
}
