// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::madt::{
    MadtEntryHeader,
    MadtEntryIoApicSourceOverride,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl MadtEntryIoApicSourceOverride {
    pub unsafe fn from_ptr(ptr: *const MadtEntryIoApicSourceOverride) -> Self {
        Self {
            header: MadtEntryHeader::from_ptr(ptr as *const MadtEntryHeader),
            bus: (*ptr).bus,
            source: (*ptr).source,
            global_sys_int: (*ptr).global_sys_int,
            flags: (*ptr).flags,
        }
    }

    pub fn display(&self) {
        info!("Local APIC Override:");
        info!("  Bus Source: {}", self.bus);
        info!("  IRQ Source: {}", self.source);
        let global_sys_int: u32 = self.global_sys_int;
        info!("  Global System Interrupt: {}", global_sys_int);
        let flags: u16 = self.flags;
        info!("  Flags: 0x{:x}", flags);
    }
}
