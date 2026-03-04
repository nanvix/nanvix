// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    mem,
    x86_64::mem::paging::{
        flags::{
            AccessedFlag,
            NoExecuteFlag,
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
// PDPT Entry Flags
//==================================================================================================

///
/// # Description
///
/// A type that represents flags of a PDPT entry.
///
#[derive(Debug)]
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
    /// No-execute flag (bit 63).
    no_execute: NoExecuteFlag,
}

impl PdptEntryFlags {
    pub fn new(
        present: PresentFlag,
        read_write: ReadWriteFlag,
        user_supervisor: UserSupervisorFlag,
        page_write_through: PageWriteThroughFlag,
        page_cache_disable: PageCacheDisableFlag,
        accessed: AccessedFlag,
        no_execute: NoExecuteFlag,
    ) -> Self {
        Self {
            present,
            read_write,
            user_supervisor,
            page_write_through,
            page_cache_disable,
            accessed,
            no_execute,
        }
    }

    fn from_raw_value(value: u64) -> Self {
        Self {
            present: PresentFlag::from_raw_value(value),
            read_write: ReadWriteFlag::from_raw_value(value),
            user_supervisor: UserSupervisorFlag::from_raw_value(value),
            page_write_through: PageWriteThroughFlag::from_raw_value(value),
            page_cache_disable: PageCacheDisableFlag::from_raw_value(value),
            accessed: AccessedFlag::from_raw_value(value),
            no_execute: NoExecuteFlag::from_raw_value(value),
        }
    }

    fn into_raw_value(self) -> u64 {
        let mut value: u64 = 0;

        value |= self.present.into_raw_value();
        value |= self.read_write.into_raw_value();
        value |= self.user_supervisor.into_raw_value();
        value |= self.page_write_through.into_raw_value();
        value |= self.page_cache_disable.into_raw_value();
        value |= self.accessed.into_raw_value();
        value |= self.no_execute.into_raw_value();

        value
    }
}

//==================================================================================================
// PDPT Entry
//==================================================================================================

///
/// # Description
///
/// A type that represents a PDPT entry (second level of 4-level paging).
/// Each PDPT entry points to a Page Directory (PD).
///
#[derive(Debug)]
pub struct PdptEntry {
    /// Flags.
    flags: PdptEntryFlags,
    /// Physical frame number of the page directory.
    frame: FrameNumber,
}

impl PdptEntry {
    pub fn new(flags: PdptEntryFlags, frame: FrameNumber) -> Self {
        Self { flags, frame }
    }

    pub fn from_raw_value(value: u64) -> Option<Self> {
        let frame_bits = (value & mem::PAGE_ENTRY_ADDR_MASK) >> mem::FRAME_SHIFT as u64;
        Some(Self {
            flags: PdptEntryFlags::from_raw_value(value),
            frame: FrameNumber::from_raw_value(frame_bits)?,
        })
    }

    pub fn into_raw_value(self) -> u64 {
        let mut value: u64 = 0;

        value |= self.flags.into_raw_value();
        value |= self.frame.into_raw_value() << mem::FRAME_SHIFT as u64;

        value
    }

    pub fn is_present(&self) -> bool {
        match self.flags.present {
            PresentFlag::Present => true,
            PresentFlag::NotPresent => false,
        }
    }

    pub fn frame(&self) -> FrameNumber {
        self.frame
    }
}
