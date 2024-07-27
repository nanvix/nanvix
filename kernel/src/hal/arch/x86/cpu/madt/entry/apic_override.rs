// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::madt::{
    MadtEntryHeader,
    MadtLocalApicAddressOverride,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl MadtLocalApicAddressOverride {
    pub unsafe fn from_ptr(ptr: *const MadtLocalApicAddressOverride) -> Self {
        Self {
            header: MadtEntryHeader::from_ptr(ptr as *const MadtEntryHeader),
            reserved: (*ptr).reserved,
            local_apic_address: (*ptr).local_apic_address,
        }
    }

    pub fn display(&self) {
        info!("Local APIC Address Override:");
        let local_apic_address: u64 = self.local_apic_address;
        info!("  Local APIC Address: 0x{:x}", local_apic_address);
    }
}
