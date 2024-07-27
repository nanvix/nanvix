// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::madt::{
    MadtEntryHeader,
    MadtEntryIoApicNmi,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl MadtEntryIoApicNmi {
    pub unsafe fn from_ptr(ptr: *const MadtEntryIoApicNmi) -> Self {
        Self {
            header: MadtEntryHeader::from_ptr(ptr as *const MadtEntryHeader),
            io_apic_id: (*ptr).io_apic_id,
            reserved: (*ptr).reserved,
            flags: (*ptr).flags,
            global_sys_int: (*ptr).global_sys_int,
        }
    }

    pub fn display(&self) {
        info!("I/O APIC NMI:");
        info!("  I/O APIC ID: {}", self.io_apic_id);
        let flags: u16 = self.flags;
        info!("  Flags: 0x{:x}", flags);
        let global_sys_int: u32 = self.global_sys_int;
        info!("  Global System Interrupt: {}", global_sys_int);
    }
}
