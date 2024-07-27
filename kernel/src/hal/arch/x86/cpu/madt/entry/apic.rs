// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::madt::{
    MadtEntryHeader,
    MadtEntryLocalApic,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl MadtEntryLocalApic {
    pub unsafe fn from_ptr(ptr: *const MadtEntryLocalApic) -> Self {
        Self {
            header: MadtEntryHeader::from_ptr(ptr as *const MadtEntryHeader),
            processor_id: (*ptr).processor_id,
            apic_id: (*ptr).apic_id,
            flags: (*ptr).flags,
        }
    }

    pub fn display(&self) {
        info!("Local APIC:");
        info!("  Processor ID: {}", self.processor_id);
        info!("  APIC ID: {}", self.apic_id);
        let flags: u32 = self.flags;
        info!("  Flags: 0x{:x}", flags);
    }
}
