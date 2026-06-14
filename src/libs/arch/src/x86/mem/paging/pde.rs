// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("pde.spec.rs");
#[cfg(verus_keep_ghost)]
include!("pde.proof.rs");

use crate::{
    mem::paging::TableEntry,
    x86::mem::paging::{
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
        PteWord,
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
#[verus_verify]
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
    #[allow(unused, verus_impl_method_marker)]
    #[verus_spec(result =>
        ensures
            result@ == spec_pde_flags_new(
                present,
                read_write,
                user_supervisor,
                page_write_through,
                page_cache_disable,
                accessed,
                dirty,
                page_size,
            ),
    )]
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
    #[verus_spec(result =>
        ensures result == self@.present,
    )]
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
    fn from_raw_value(value: PteWord) -> Self {
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
    fn into_raw_value(self) -> PteWord {
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
}

//==================================================================================================
// Page Directory Entry
//==================================================================================================

///
/// # Description
///
/// A type that represents a page directory entry.
///
#[verus_verify]
#[derive(Debug, Clone, Copy)]
pub struct PageDirectoryEntry {
    /// Flags.
    flags: PageDirectoryEntryFlags,
    /// Physical address of the page table (or large page).
    frame: FrameNumber,
}

impl PageDirectoryEntry {
    /// Size in bytes of the hardware page directory entry representation (32-bit encoded value).
    pub const SIZE: usize = ::core::mem::size_of::<PteWord>();

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
    #[allow(unused, verus_impl_method_marker)]
    #[verus_spec(result =>
        ensures
            result@ == spec_pde_new(flags@, frame@),
            result.inv(),
    )]
    pub fn new(flags: PageDirectoryEntryFlags, frame: FrameNumber) -> Self {
        proof! { use_type_invariant(frame); }
        Self { flags, frame }
    }

    ///
    /// # Description
    ///
    /// Constructs a [`PageDirectoryEntry`] from a raw 32-bit value.
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
        Some(Self {
            flags: PageDirectoryEntryFlags::from_raw_value(value),
            frame: FrameNumber::from_raw_value(value as usize >> crate::mem::FRAME_SHIFT)?,
        })
    }

    ///
    /// # Description
    ///
    /// Converts a [`PageDirectoryEntry`] into a raw 32-bit value.
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
    #[verus_spec(result =>
        ensures result == self@.flags.present,
    )]
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
    #[verus_spec(result =>
        ensures
            result as int == self@.frame * (crate::mem::FRAME_SIZE as int),
            result as int % (crate::mem::FRAME_SIZE as int) == 0,
    )]
    pub fn frame_address(&self) -> usize {
        proof! { admit(); }
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
}

impl TableEntry for PageDirectoryEntry {
    fn from_raw(raw: PteWord) -> Option<Self> {
        Self::from_raw_value(raw)
    }

    fn raw(self) -> PteWord {
        self.into_raw_value()
    }
}
