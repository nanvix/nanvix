// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::madt::{
    MadtEntryHeader,
    MadtEntryIoApic,
};

//==================================================================================================
// Implementations
//==================================================================================================

impl MadtEntryIoApic {
    pub unsafe fn from_ptr(ptr: *const MadtEntryIoApic) -> Self {
        Self {
            header: MadtEntryHeader::from_ptr(ptr as *const MadtEntryHeader),
            io_apic_id: (*ptr).io_apic_id,
            reserved: (*ptr).reserved,
            io_apic_addr: (*ptr).io_apic_addr,
            global_sys_int_base: (*ptr).global_sys_int_base,
        }
    }

    pub fn display(&self) {
        info!("I/O APIC:");
        info!("  I/O APIC ID: {}", self.io_apic_id);
        info!("  Reserved: {}", self.reserved);
        let io_apic_addr: u32 = self.io_apic_addr;
        info!("  I/O APIC Address: 0x{:x}", io_apic_addr);
        let global_sys_int_base: u32 = self.global_sys_int_base;
        info!("  Global System Interrupt Base: {}", global_sys_int_base);
    }
}
