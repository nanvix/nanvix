// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::collections::LinkedList;
use ::arch::cpu::{
    acpi::AcpiSdtHeader,
    madt::{
        Madt,
        MadtEntryHeader,
        MadtEntryIoApic,
        MadtEntryIoApicNmi,
        MadtEntryIoApicSourceOverride,
        MadtEntryLocalApic,
        MadtEntryLocalApicNmi,
        MadtEntryLocalX2Apic,
        MadtEntryType,
        MadtLocalApicAddressOverride,
    },
};
use ::sys::error::Error;

pub struct MadtInfo {
    pub sdt: AcpiSdtHeader,
    pub local_apic_addr: u32,
    pub flags: u32,
    pub entries: LinkedList<MadtEntry>,
}

pub enum MadtEntry {
    LocalApic(MadtEntryLocalApic),
    IoApicSourceOverride(MadtEntryIoApicSourceOverride),
    IoApic(MadtEntryIoApic),
    IoApicNmi(MadtEntryIoApicNmi),
    LocalApicNmi(MadtEntryLocalApicNmi),
    LocalApicAddressOverride(MadtLocalApicAddressOverride),
    LocalX2Apic(MadtEntryLocalX2Apic),
}

impl MadtInfo {
    pub unsafe fn from_ptr(ptr: *const Madt) -> Result<Self, Error> {
        Ok(Self {
            sdt: AcpiSdtHeader::from_ptr(ptr as *const AcpiSdtHeader).unwrap(),
            local_apic_addr: ptr.read_unaligned().local_apic_addr,
            flags: ptr.read_unaligned().flags,
            entries: LinkedList::new(),
        })
    }

    pub fn display(&self) {
        info!("MADT:");
        info!("  Local APIC Address: 0x{:x}", self.local_apic_addr);
        info!("  Flags: 0x{:x}", self.flags);
    }

    ///
    /// # Description
    ///
    /// Checks if the system has a dual-8259 setup.
    ///
    /// # Return Values
    ///
    /// If the system has a dual-8259 setup, `true` is returned. Otherwise, `false` is returned.
    ///
    pub const fn has_8259_pic(&self) -> bool {
        self.flags & Madt::PCAT_COMPAT != 0
    }

    ///
    /// # Description
    ///
    /// Retrieves information about the I/O APIC.
    ///
    /// # Return Values
    ///
    /// If the I/O APIC is present, a reference to its information is returned. Otherwise, `None` is
    /// returned instead
    ///
    pub fn get_ioapic_info(&self) -> Option<&MadtEntryIoApic> {
        // TODO: handle multiple I/O APICs.
        for entry in self.entries.iter() {
            match entry {
                MadtEntry::IoApic(ioapic_info) => return Some(ioapic_info),
                _ => (),
            }
        }
        None
    }

    ///
    /// # Description
    ///
    /// Retrieves information about the local APIC.
    ///
    /// # Return Values
    ///
    /// If the local APIC is present, a reference to its information is returned. Otherwise, `None`
    /// is returned instead.
    ///
    pub fn get_lapic_info(&self) -> Option<&MadtEntryLocalApic> {
        // TODO: handle multiple local APICs.
        for entry in self.entries.iter() {
            match entry {
                MadtEntry::LocalApic(local_apic_info) => return Some(local_apic_info),
                _ => (),
            }
        }
        None
    }

    ///
    /// # Description
    ///
    /// Retrieves information about interrupt source override.
    ///
    /// # Return Values
    ///
    /// A list of interrupt source override information.
    ///
    pub fn get_ioapic_source_override(&self) -> LinkedList<&MadtEntryIoApicSourceOverride> {
        let mut ioapic_source_override: LinkedList<&MadtEntryIoApicSourceOverride> =
            LinkedList::new();
        for e in self.entries.iter() {
            if let MadtEntry::IoApicSourceOverride(ioapic_source_override_info) = e {
                ioapic_source_override.push_back(ioapic_source_override_info)
            }
        }
        ioapic_source_override
    }
}

pub unsafe fn parse(madt: *const Madt) -> Result<MadtInfo, Error> {
    let base: *const u8 = madt as *const u8;
    let base: *const u8 = base.add(core::mem::size_of::<Madt>());
    let madt_len: usize = madt.read_unaligned().header.length as usize;
    let mut offset: usize = 0;

    let mut madt: MadtInfo = MadtInfo::from_ptr(madt)?;

    while offset < madt_len as usize {
        let ptr: *const MadtEntryHeader = unsafe { base.add(offset) as *const MadtEntryHeader };
        let header: MadtEntryHeader = MadtEntryHeader::from_ptr(ptr);

        match MadtEntryType::from(header.typ) {
            MadtEntryType::LocalApic => {
                let entry: *const MadtEntryLocalApic =
                    unsafe { base.add(offset) as *const MadtEntryLocalApic };
                let local_apic_info: MadtEntryLocalApic = MadtEntryLocalApic::from_ptr(entry);
                madt.entries
                    .push_back(MadtEntry::LocalApic(local_apic_info));
            },
            MadtEntryType::IoApic => {
                let entry: *const MadtEntryIoApic =
                    unsafe { base.add(offset) as *const MadtEntryIoApic };
                let ioapic_info: MadtEntryIoApic = MadtEntryIoApic::from_ptr(entry);
                madt.entries.push_back(MadtEntry::IoApic(ioapic_info));
            },
            MadtEntryType::IoApicSourceOverride => {
                let entry: *const MadtEntryIoApicSourceOverride =
                    unsafe { base.add(offset) as *const MadtEntryIoApicSourceOverride };
                let ioapic_override_info: MadtEntryIoApicSourceOverride =
                    MadtEntryIoApicSourceOverride::from_ptr(entry);
                madt.entries
                    .push_back(MadtEntry::IoApicSourceOverride(ioapic_override_info));
            },
            MadtEntryType::IoApicNmi => {
                let entry: *const MadtEntryIoApicNmi =
                    unsafe { base.add(offset) as *const MadtEntryIoApicNmi };
                let ioapic_nmi_info: MadtEntryIoApicNmi = MadtEntryIoApicNmi::from_ptr(entry);
                madt.entries
                    .push_back(MadtEntry::IoApicNmi(ioapic_nmi_info));
            },
            MadtEntryType::LocalApicNmi => {
                let entry: *const MadtEntryLocalApicNmi =
                    unsafe { base.add(offset) as *const MadtEntryLocalApicNmi };
                let local_apic_nmi_info: MadtEntryLocalApicNmi =
                    MadtEntryLocalApicNmi::from_ptr(entry);
                madt.entries
                    .push_back(MadtEntry::LocalApicNmi(local_apic_nmi_info));
            },
            MadtEntryType::LocalApicAddressOverride => {
                let entry: *const MadtLocalApicAddressOverride =
                    unsafe { base.add(offset) as *const MadtLocalApicAddressOverride };
                let local_apic_address_override_info: MadtLocalApicAddressOverride =
                    MadtLocalApicAddressOverride::from_ptr(entry);
                madt.entries.push_back(MadtEntry::LocalApicAddressOverride(
                    local_apic_address_override_info,
                ));
            },
            MadtEntryType::LocalX2Apic => {
                let entry: *const MadtEntryLocalX2Apic =
                    unsafe { base.add(offset) as *const MadtEntryLocalX2Apic };
                let local_x2apic_info: MadtEntryLocalX2Apic = MadtEntryLocalX2Apic::from_ptr(entry);
                madt.entries
                    .push_back(MadtEntry::LocalX2Apic(local_x2apic_info));
            },
        }
        offset += unsafe { (*ptr).len as usize };
    }

    Ok(madt)
}
