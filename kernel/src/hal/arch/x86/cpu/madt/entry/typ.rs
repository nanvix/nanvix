// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::madt::MadtEntryType;

//==================================================================================================
// Implementations
//==================================================================================================

impl From<u8> for MadtEntryType {
    fn from(entry_type: u8) -> Self {
        match entry_type {
            0 => MadtEntryType::LocalApic,
            1 => MadtEntryType::IoApic,
            2 => MadtEntryType::IoApicSourceOverride,
            3 => MadtEntryType::IoApicNmi,
            4 => MadtEntryType::LocalApicNmi,
            5 => MadtEntryType::LocalApicAddressOverride,
            9 => MadtEntryType::LocalX2Apic,
            _ => MadtEntryType::LocalApic,
        }
    }
}
