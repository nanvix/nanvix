// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::madt::{
    MadtEntryHeader,
    MadtEntryLocalX2Apic,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl MadtEntryLocalX2Apic {
    pub unsafe fn from_ptr(ptr: *const MadtEntryLocalX2Apic) -> Self {
        Self {
            header: MadtEntryHeader::from_ptr(ptr as *const MadtEntryHeader),
            reserved: (*ptr).reserved,
            x2apic_id: (*ptr).x2apic_id,
            flags: (*ptr).flags,
            acpi_processor_uid: (*ptr).acpi_processor_uid,
        }
    }

    pub fn display(&self) {
        info!("Local x2APIC:");
        let x2apic_id: u32 = self.x2apic_id;
        info!("  x2APIC ID: {}", x2apic_id);
        let flags: u32 = self.flags;
        info!("  Flags: 0x{:x}", flags);
        let acpi_processor_uid: u32 = self.acpi_processor_uid;
        info!("  ACPI Processor UID: {}", acpi_processor_uid);
    }
}
