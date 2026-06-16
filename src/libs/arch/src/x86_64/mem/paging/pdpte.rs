// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    mem,
    x86::mem::paging::{
        AccessedFlag,
        DirtyFlag,
        FrameNumber,
        PageCacheDisableFlag,
        PageTableEntry,
        PageTableEntryFlags,
        PageWriteThroughFlag,
        PresentFlag,
        ReadWriteFlag,
        TableEntry,
        UserSupervisorFlag,
        PHYS_ADDR_MASK,
    },
};

//==================================================================================================
// PDPT Entry Flags
//==================================================================================================

///
/// # Description
///
/// A type that represents flags of a PDPT entry.
///
#[derive(Clone, Copy, Debug)]
pub struct PdptEntryFlags {
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

impl PdptEntryFlags {
    ///
    /// # Description
    ///
    /// Constructs a [`PdptEntryFlags`] with the given flags.
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

    /// Checks if the present flag is set.
    #[inline(always)]
    pub fn is_present(&self) -> bool {
        matches!(self.present, PresentFlag::Present)
    }

    /// Checks if the user flag is set.
    #[inline(always)]
    pub fn is_user(&self) -> bool {
        matches!(self.user_supervisor, UserSupervisorFlag::User)
    }

    /// Sets user/supervisor flag.
    #[inline(always)]
    pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag) {
        self.user_supervisor = user_supervisor;
    }

    /// Constructs a [`PdptEntryFlags`] from a raw value.
    fn from_raw_value(value: u64) -> Self {
        // Flag extractors take PteWord (u64 on x86_64); flags occupy the low bits.
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

    /// Converts a [`PdptEntryFlags`] into a raw value.
    fn into_raw_value(self) -> u64 {
        let mut v: u64 = 0;
        v |= self.present.into_raw_value();
        v |= self.read_write.into_raw_value();
        v |= self.user_supervisor.into_raw_value();
        v |= self.page_write_through.into_raw_value();
        v |= self.page_cache_disable.into_raw_value();
        v |= self.accessed.into_raw_value();
        v |= self.dirty.into_raw_value();
        v
    }
}

impl From<PageTableEntryFlags> for PdptEntryFlags {
    fn from(flags: PageTableEntryFlags) -> Self {
        Self::from_raw_value(flags.into_raw_value() as u64)
    }
}

//==================================================================================================
// PDPT Entry
//==================================================================================================

///
/// # Description
///
/// A type that represents a PDPT (Page Directory Pointer Table) entry.
///
#[derive(Debug, Clone, Copy)]
pub struct PdptEntry {
    /// Flags.
    flags: PdptEntryFlags,
    /// Frame number (extracted from the physical-address field).
    frame: FrameNumber,
}

impl PdptEntry {
    /// Size in bytes of the hardware PDPT entry representation (64-bit encoded value).
    pub const SIZE: usize = PageTableEntry::SIZE;

    /// Constructs a [`PdptEntry`] with the given flags and frame number.
    pub fn new(flags: PdptEntryFlags, frame: FrameNumber) -> Self {
        Self { flags, frame }
    }

    /// Constructs a [`PdptEntry`] from a raw 64-bit value.
    pub fn from_raw_value(value: u64) -> Option<Self> {
        Some(Self {
            flags: PdptEntryFlags::from_raw_value(value),
            frame: FrameNumber::from_raw_value(
                ((value & PHYS_ADDR_MASK) >> mem::FRAME_SHIFT) as usize,
            )?,
        })
    }

    /// Converts a [`PdptEntry`] into a raw 64-bit value.
    pub fn into_raw_value(self) -> u64 {
        let mut value: u64 = self.flags.into_raw_value();
        value |= ((self.frame.into_raw_value() << mem::FRAME_SHIFT) as u64) & PHYS_ADDR_MASK;
        value
    }

    /// Returns the flags.
    pub fn flags(&self) -> PdptEntryFlags {
        self.flags
    }

    /// Returns the frame number.
    pub fn frame_number(&self) -> FrameNumber {
        self.frame
    }

    /// Returns the physical address of the pointed-to page directory.
    pub fn frame_address(&self) -> usize {
        self.frame.into_raw_value() << mem::FRAME_SHIFT
    }

    /// Checks if the entry is present.
    pub fn is_present(&self) -> bool {
        self.flags.is_present()
    }

    /// Returns a copy with the user flag set if `user` is `true` and not already set.
    pub fn ensure_user(mut self, user: bool) -> Self {
        if user && !self.flags.is_user() {
            self.flags.set_user_supervisor(UserSupervisorFlag::User);
        }
        self
    }

    /// Sets the user/supervisor flag.
    pub fn set_user_supervisor(&mut self, user_supervisor: UserSupervisorFlag) {
        self.flags.set_user_supervisor(user_supervisor);
    }
}

impl TableEntry for PdptEntry {
    fn from_raw(raw: u64) -> Option<Self> {
        Self::from_raw_value(raw)
    }

    fn raw(self) -> u64 {
        self.into_raw_value()
    }
}
