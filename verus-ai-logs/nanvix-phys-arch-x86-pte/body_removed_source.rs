// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("pte.spec.rs");
#[cfg(verus_keep_ghost)]
include!("pte.proof.rs");

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
    ) -> Self { ... }

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
    fn from_raw_value(value: PteWord) -> Self { ... }

    ///
    /// # Description
    ///
    /// Converts a [`PageTableEntryFlags`] into a raw value.
    ///
    /// # Returns
    ///
    /// The raw value.
    ///
    fn into_raw_value(self) -> PteWord { ... }

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
    pub fn is_present(&self) -> bool { ... }

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
    pub fn is_user(&self) -> bool { ... }

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
    pub fn is_writable(&self) -> bool { ... }

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
    pub fn set_read_write(&mut self, read_write: ReadWriteFlag) { ... }

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
    pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag) { ... }

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
    pub fn is_cow(&self) -> bool { ... }

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
    pub fn set_cow(&mut self, cow: CopyOnWriteFlag) { ... }
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
    pub fn new(flags: PageTableEntryFlags, frame: FrameNumber) -> Self { ... }

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
    pub fn from_raw_value(value: PteWord) -> Option<Self> { ... }

    ///
    /// # Description
    ///
    /// Converts a [`PageTableEntry`] into a raw value.
    ///
    /// # Returns
    ///
    /// The raw value.
    ///
    pub fn into_raw_value(self) -> PteWord { ... }

    ///
    /// # Description
    ///
    /// Returns the flags associated with the target page table entry.
    ///
    /// # Returns
    ///
    /// The flags.
    ///
    pub fn flags(&self) -> PageTableEntryFlags { ... }

    ///
    /// # Description
    ///
    /// Returns the frame number associated with the target page table entry.
    ///
    /// # Returns
    ///
    /// The frame number associated with the target page table entry.
    ///
    pub fn frame_number(&self) -> FrameNumber { ... }

    ///
    /// # Description
    ///
    /// Returns the physical address (frame number × frame size) of the page frame.
    ///
    /// # Returns
    ///
    /// The physical address.
    ///
    pub fn frame_address(&self) -> usize { ... }

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
    pub fn is_present(&self) -> bool { ... }

    ///
    /// # Description
    ///
    /// Sets read/write flag in the target page.
    ///
    /// # Parameters
    ///
    /// - `read_write`: The read/write flag.
    ///
    pub fn set_read_write(&mut self, read_write: ReadWriteFlag) { ... }

    ///
    /// # Description
    ///
    /// Sets user/supervisor flag in the target page.
    ///
    /// # Parameters
    ///
    /// - `user_supervisor`: The user/supervisor flag.
    ///
    pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag) { ... }

    ///
    /// # Description
    ///
    /// Checks whether the target page table entry is marked copy-on-write.
    ///
    /// # Returns
    ///
    /// `true` if the entry is marked copy-on-write, `false` otherwise.
    ///
    pub fn is_cow(&self) -> bool { ... }

    ///
    /// # Description
    ///
    /// Sets the copy-on-write flag in the target page table entry.
    ///
    /// # Parameters
    ///
    /// - `cow`: The copy-on-write flag.
    ///
    pub fn set_cow(&mut self, cow: CopyOnWriteFlag) { ... }
}

impl TableEntry for PageTableEntry {
    fn from_raw(raw: PteWord) -> Option<Self> { ... }

    fn raw(self) -> PteWord { ... }
}
