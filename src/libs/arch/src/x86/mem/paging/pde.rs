// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::x86::mem::paging::{
    flags::{
        AccessedFlag,
        DirtyFlag,
        PageCacheDisableFlag,
        PageSizeFlag,
        PageWriteThroughFlag,
        PresentFlag,
        ReadWriteFlag,
        UserSupervisorFlag,
    },
    frame::FrameNumber,
    pte::PageTableEntryFlags,
    PteWord,
};

//==================================================================================================
// Page Directory Entry Flags
//==================================================================================================

///
/// # Description
///
/// A type that represents flags of a page directory entry.
///
#[derive(Clone, Copy, Debug)]
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
    /// Page size flag (PDE-only, bit 7).
    page_size: PageSizeFlag,
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
    /// - `page_size`: The page size flag.
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
        page_size: PageSizeFlag,
    ) -> Self {
        Self {
            present,
            read_write,
            user_supervisor,
            page_write_through,
            page_cache_disable,
            accessed,
            dirty,
            page_size,
        }
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
    /// Checks if the page size flag is set (large page).
    ///
    /// # Returns
    ///
    /// `true` if the page size flag is set, `false` otherwise.
    ///
    #[inline(always)]
    pub fn is_large_page(&self) -> bool {
        matches!(self.page_size, PageSizeFlag::Large)
    }

    ///
    /// # Description
    ///
    /// Sets page size.
    ///
    /// # Parameters
    ///
    /// - `page_size`: The page size flag.
    ///
    #[inline(always)]
    pub fn set_page_size(&mut self, page_size: PageSizeFlag) {
        self.page_size = page_size;
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
    pub(crate) fn from_raw_value(value: PteWord) -> Self {
        Self {
            present: PresentFlag::from_raw_value(value),
            read_write: ReadWriteFlag::from_raw_value(value),
            user_supervisor: UserSupervisorFlag::from_raw_value(value),
            page_write_through: PageWriteThroughFlag::from_raw_value(value),
            page_cache_disable: PageCacheDisableFlag::from_raw_value(value),
            accessed: AccessedFlag::from_raw_value(value),
            dirty: DirtyFlag::from_raw_value(value),
            page_size: PageSizeFlag::from_raw_value(value),
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
    pub(crate) fn into_raw_value(self) -> PteWord {
        let mut value: PteWord = 0;

        value |= self.present.into_raw_value();
        value |= self.read_write.into_raw_value();
        value |= self.user_supervisor.into_raw_value();
        value |= self.page_write_through.into_raw_value();
        value |= self.page_cache_disable.into_raw_value();
        value |= self.accessed.into_raw_value();
        value |= self.dirty.into_raw_value();
        value |= self.page_size.into_raw_value();

        value
    }

    /// Converts PDE flags to PTE flags (dropping the page-size flag).
    pub fn to_pte_flags(&self) -> PageTableEntryFlags {
        PageTableEntryFlags::new(
            self.present,
            self.read_write,
            self.user_supervisor,
            self.page_write_through,
            self.page_cache_disable,
            self.accessed,
            self.dirty,
        )
    }
}

impl From<PageTableEntryFlags> for PageDirectoryEntryFlags {
    /// Converts PTE flags to PDE flags with [`PageSizeFlag::Standard`].
    fn from(f: PageTableEntryFlags) -> Self {
        // Round-trip through raw value to avoid accessing private PTE fields.
        let mut raw: PteWord = f.into_raw_value();
        // Clear the PS bit (bit 7) to ensure Standard page size.
        raw &= !(1 << 7);
        Self::from_raw_value(raw)
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
#[derive(Debug, Clone, Copy)]
pub struct PageDirectoryEntry {
    /// Flags.
    flags: PageDirectoryEntryFlags,
    /// Physical address of the page table (or large page).
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
    /// Returns the flags associated with the target page directory entry.
    ///
    /// # Returns
    ///
    /// The flags.
    ///
    pub fn flags(&self) -> PageDirectoryEntryFlags {
        self.flags
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
        self.flags.is_present()
    }

    ///
    /// # Description
    ///
    /// Returns the frame number of the target page directory entry.
    ///
    /// # Returns
    ///
    /// The frame number.
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
    /// Checks if the page size flag is set.
    ///
    /// # Returns
    ///
    /// `true` if the page size flag is set, `false` otherwise.
    ///
    pub fn is_large_page(&self) -> bool {
        self.flags.is_large_page()
    }

    ///
    /// # Description
    ///
    /// Sets page size.
    ///
    /// # Parameters
    ///
    /// - `page_size`: The page size flag.
    ///
    pub fn set_page_size(&mut self, page_size: PageSizeFlag) {
        self.flags.set_page_size(page_size);
    }

    ///
    /// # Description
    ///
    /// Sets read/write flag in the target page directory entry.
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
    /// Sets user/supervisor flag in the target page directory entry.
    ///
    /// # Parameters
    ///
    /// - `user_supervisor`: The user/supervisor flag.
    ///
    pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag) {
        self.flags.set_user_supervisor(user_supervisor);
    }

    /// Returns a copy with the user flag set if `user` is `true` and not already set.
    pub fn ensure_user(mut self, user: bool) -> Self {
        if user && !self.flags.is_user() {
            self.flags.set_user_supervisor(UserSupervisorFlag::User);
        }
        self
    }

    /// Returns the PDE flags converted to PTE-compatible flags (without page size).
    pub fn flags_without_ps(&self) -> PageTableEntryFlags {
        self.flags.to_pte_flags()
    }
}

//==================================================================================================
// Raw Value Serialization
//==================================================================================================

impl PageDirectoryEntry {
    /// Size in bytes of the hardware page directory entry representation.
    pub const SIZE: usize = ::core::mem::size_of::<PteWord>();

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
    pub fn from_raw_value(value: PteWord) -> Option<Self> {
        use crate::x86::mem::paging::PHYS_ADDR_MASK;
        Some(Self {
            flags: PageDirectoryEntryFlags::from_raw_value(value),
            frame: FrameNumber::from_raw_value(
                (value & PHYS_ADDR_MASK) as usize >> crate::mem::FRAME_SHIFT,
            )?,
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
    pub fn into_raw_value(self) -> PteWord {
        let mut value: PteWord = 0;

        value |= self.flags.into_raw_value();
        value |= (self.frame.into_raw_value() << crate::mem::FRAME_SHIFT) as PteWord;

        value
    }

    /// Returns the physical base address of a large page.
    pub fn large_page_address(&self) -> u64 {
        use crate::x86::mem::paging::LARGE_PAGE_ADDR_MASK;
        (self.into_raw_value() as u64) & (LARGE_PAGE_ADDR_MASK as u64)
    }
}

impl crate::x86::mem::paging::table::TableEntry for PageDirectoryEntry {
    fn from_raw(raw: PteWord) -> Option<Self> {
        Self::from_raw_value(raw)
    }

    fn raw(self) -> PteWord {
        self.into_raw_value()
    }
}
