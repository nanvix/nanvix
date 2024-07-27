// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::madt::{
    MadtEntryHeader,
    MadtEntryLocalApicNmi,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl MadtEntryLocalApicNmi {
    pub unsafe fn from_ptr(ptr: *const MadtEntryLocalApicNmi) -> Self {
        Self {
            header: MadtEntryHeader::from_ptr(ptr as *const MadtEntryHeader),
            processor_id: (*ptr).processor_id,
            flags: (*ptr).flags,
            local_apic_lint: (*ptr).local_apic_lint,
        }
    }

    pub fn display(&self) {
        info!("Local APIC NMI:");
        info!("  Processor ID: {}", self.processor_id);
        let flags: u16 = self.flags;
        info!("  Flags: 0x{:x}", flags);
        let local_apic_lint: u8 = self.local_apic_lint;
        info!("  Local APIC LINT: {}", local_apic_lint);
    }
}
