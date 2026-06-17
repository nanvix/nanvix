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

#[verus_verify]
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
    ) -> Self { ... }
}

impl PageDirectoryEntryFlags {
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
    /// Checks if the page size flag is set (large page).
    ///
    /// # Returns
    ///
    /// `true` if the page size flag is set, `false` otherwise.
    ///
    #[inline(always)]
    pub fn is_large_page(&self) -> bool { ... }

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
    pub fn set_page_size(&mut self, page_size: PageSizeFlag) { ... }

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
    fn from_raw_value(value: PteWord) -> Self { ... }

    ///
    /// # Description
    ///
    /// Converts a [`PageDirectoryEntryFlags`] into a raw value.
    ///
    /// # Returns
    ///
    /// The raw value.
    ///
    fn into_raw_value(self) -> PteWord { ... }
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
}

#[verus_verify]
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
    #[verus_spec(result =>
        ensures
            result@ == spec_pde_new(flags@, frame@),
            result.inv(),
    )]
    pub fn new(flags: PageDirectoryEntryFlags, frame: FrameNumber) -> Self { ... }
}

impl PageDirectoryEntry {
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
    pub fn from_raw_value(value: PteWord) -> Option<Self> { ... }

    ///
    /// # Description
    ///
    /// Converts a [`PageDirectoryEntry`] into a raw 32-bit value.
    ///
    /// # Returns
    ///
    /// The raw value.
    ///
    pub fn into_raw_value(self) -> PteWord { ... }

    ///
    /// # Description
    ///
    /// Returns the flags associated with the target page directory entry.
    ///
    /// # Returns
    ///
    /// The flags.
    ///
    pub fn flags(&self) -> PageDirectoryEntryFlags { ... }

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
    pub fn is_present(&self) -> bool { ... }

    ///
    /// # Description
    ///
    /// Returns the frame number of the target page directory entry.
    ///
    /// # Returns
    ///
    /// The frame number.
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
    #[verus_spec(result =>
        ensures
            result as int == self@.frame * (crate::mem::FRAME_SIZE as int),
            result as int % (crate::mem::FRAME_SIZE as int) == 0,
    )]
    // VERUS REWRITE: the original `self.frame.into_raw_value() << crate::mem::FRAME_SHIFT`
    // (single expression) is split so the `into_raw_value()` postcondition
    // (`0 <= self@ <= FrameNumber::spec_max()`) lands in context *before* the overflow-bearing
    // shift, and `lemma_frame_address(raw)` can be invoked between them to discharge the
    // no-overflow + `FRAME_SIZE`-alignment `ensures`. The operand must be named (`let raw`)
    // because an exec call cannot appear inside `proof!`, so there is otherwise no point between
    // the call and the shift to invoke the lemma. Same value, same operations, same time/space
    // complexity — semantically equivalent.
    // Reproducer: verus-ai-logs/nanvix-phys-arch-x86-pde/cheating-elimination/repro/frame_address.rs
    pub fn frame_address(&self) -> usize { ... }

    ///
    /// # Description
    ///
    /// Checks if the page size flag is set.
    ///
    /// # Returns
    ///
    /// `true` if the page size flag is set, `false` otherwise.
    ///
    pub fn is_large_page(&self) -> bool { ... }

    ///
    /// # Description
    ///
    /// Sets page size.
    ///
    /// # Parameters
    ///
    /// - `page_size`: The page size flag.
    ///
    pub fn set_page_size(&mut self, page_size: PageSizeFlag) { ... }

    ///
    /// # Description
    ///
    /// Sets read/write flag in the target page directory entry.
    ///
    /// # Parameters
    ///
    /// - `read_write`: The read/write flag.
    ///
    pub fn set_read_write(&mut self, read_write: ReadWriteFlag) { ... }

    ///
    /// # Description
    ///
    /// Sets user/supervisor flag in the target page directory entry.
    ///
    /// # Parameters
    ///
    /// - `user_supervisor`: The user/supervisor flag.
    ///
    pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag) { ... }
}

impl TableEntry for PageDirectoryEntry {
    fn from_raw(raw: PteWord) -> Option<Self> { ... }

    fn raw(self) -> PteWord { ... }
}
