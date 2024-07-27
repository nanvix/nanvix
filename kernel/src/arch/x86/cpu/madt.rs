// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::acpi::AcpiSdtHeader;

//==================================================================================================
// MADT
//==================================================================================================

///
/// # Description
///
/// A structure that describes the Multiple APIC Description Table (MADT).
///
#[repr(C, packed)]
pub struct Madt {
    /// Standard ACPI System Description Table header
    pub header: AcpiSdtHeader,
    /// Physical address at which each processor can access its local APIC
    pub local_apic_addr: u32,
    /// Flags.
    pub flags: u32,
    /// Start of the MADT entries
    pub entries: [u32; 0], // This is a zero-length array used for variable-sized struct
}

static_assert_size!(Madt, Madt::_SIZE);

impl Madt {
    /// Size of the MADT table.
    const _SIZE: usize = core::mem::size_of::<AcpiSdtHeader>() + 8;

    /// PC-AT compatible flag.
    /// If set, the system also has a PCT-AT compatible dual-8259 setup.
    pub const PCAT_COMPAT: u32 = 1 << 0;
}

//==================================================================================================
// MADT Entry Type
//==================================================================================================

///
/// # Description
///
/// An enumeration that defines the types of entries in the MADT table
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MadtEntryType {
    LocalApic = 0,
    IoApic = 1,
    IoApicSourceOverride = 2,
    IoApicNmi = 3,
    LocalApicNmi = 4,
    LocalApicAddressOverride = 5,
    LocalX2Apic = 9,
}

//==================================================================================================
// MADT Entry Header
//==================================================================================================

///
/// # Description
///
/// A structure that describes the header of a MADT entry.
///
#[repr(C, packed)]
pub struct MadtEntryHeader {
    pub typ: u8,
    /// Length includes this header + body.
    pub len: u8,
}

impl MadtEntryHeader {
    /// Size of the entry header.
    const _SIZE: usize = 2;
}

static_assert_size!(MadtEntryHeader, MadtEntryHeader::_SIZE);

//==================================================================================================
// MADT Local APIC Entry
//==================================================================================================

///
/// # Description
///
/// A structure that describes a Local APIC entry in the MADT.
///
#[repr(C, packed)]
pub struct MadtEntryLocalApic {
    pub header: MadtEntryHeader,
    pub processor_id: u8,
    pub apic_id: u8,
    pub flags: u32,
}

impl MadtEntryLocalApic {
    /// Size of the Local APIC entry.
    const _SIZE: usize = MadtEntryHeader::_SIZE + 6;

    ///
    /// Enabled flag.
    ///
    /// If this bit is set the processor is ready for use. If this bit is clear and
    /// the Online Capable bit is set, system hardware supports enabling this
    /// processor during OS runtime. If this bit is clear and the Online Capable bit
    /// is also clear, this processor is unusable, and OSPM shall ignore the
    /// contents of the Processor Local APIC Structure.
    pub const ENABLED: u32 = 1 << 0;

    ///
    /// Online Capable flag.
    ///
    /// The information conveyed by this bit depends on the value of the Enabled
    /// bit. If the Enabled bit is set, this bit is reserved and must be zero.
    /// Otherwise, if this this bit is set, system hardware supports enabling
    /// this processor during OS runtime.
    ///
    pub const ONLINE_CAPABLE: u32 = 1 << 1;
}

static_assert_size!(MadtEntryLocalApic, MadtEntryLocalApic::_SIZE);

//==================================================================================================
// MADT IO APIC Entry
//==================================================================================================

///
/// # Description
///
/// A structure that describes an I/O APIC entry in the MADT.
///
#[repr(C, packed)]
pub struct MadtEntryIoApic {
    pub header: MadtEntryHeader,
    pub io_apic_id: u8,
    pub reserved: u8,
    pub io_apic_addr: u32,
    pub global_sys_int_base: u32,
}

impl MadtEntryIoApic {
    /// Size of the I/O APIC entry.
    const _SIZE: usize = MadtEntryHeader::_SIZE + 10;
}

static_assert_size!(MadtEntryIoApic, MadtEntryIoApic::_SIZE);

//==================================================================================================
// MADT Source Override Entry
//==================================================================================================

///
/// # Description
///
/// A structure that describes a source override entry in the MADT.
///
#[repr(C, packed)]
pub struct MadtEntryIoApicSourceOverride {
    pub header: MadtEntryHeader,
    pub bus: u8,
    pub source: u8,
    pub global_sys_int: u32,
    pub flags: u16,
}

impl MadtEntryIoApicSourceOverride {
    /// Size of the Source Override entry.
    const _SIZE: usize = MadtEntryHeader::_SIZE + 8;
}

static_assert_size!(MadtEntryIoApicSourceOverride, MadtEntryIoApicSourceOverride::_SIZE);

//==================================================================================================
// MADT NMI Entry
//==================================================================================================

///
/// # Description
///
/// A structure that describes an NMI entry in the MADT.
///
#[repr(C, packed)]
pub struct MadtEntryIoApicNmi {
    pub header: MadtEntryHeader,
    pub io_apic_id: u8,
    pub reserved: u8,
    pub flags: u16,
    pub global_sys_int: u32,
}

impl MadtEntryIoApicNmi {
    /// Size of the NMI entry.
    const _SIZE: usize = MadtEntryHeader::_SIZE + 8;
}

static_assert_size!(MadtEntryIoApicNmi, MadtEntryIoApicNmi::_SIZE);

//==================================================================================================
// MADT Local APIC NMI Entry
//==================================================================================================

///
/// # Description
///
/// A structure that describes a Local APIC NMI entry in the MADT.
///
#[repr(C, packed)]
pub struct MadtEntryLocalApicNmi {
    pub header: MadtEntryHeader,
    pub processor_id: u8,
    pub flags: u16,
    pub local_apic_lint: u8,
}

impl MadtEntryLocalApicNmi {
    /// Size of the Local APIC NMI entry.
    const _SIZE: usize = MadtEntryHeader::_SIZE + 4;
}

static_assert_size!(MadtEntryLocalApicNmi, MadtEntryLocalApicNmi::_SIZE);

//==================================================================================================
// MADT Local X2APIC Entry
//==================================================================================================

///
/// # Description
///
/// A structure that describes a Local X2APIC entry in the MADT.
///
#[repr(C, packed)]
pub struct MadtLocalApicAddressOverride {
    pub header: MadtEntryHeader,
    pub reserved: u16,
    pub local_apic_address: u64,
}

impl MadtLocalApicAddressOverride {
    /// Size of the Local X2APIC entry.
    const _SIZE: usize = MadtEntryHeader::_SIZE + 10;
}

static_assert_size!(MadtLocalApicAddressOverride, MadtLocalApicAddressOverride::_SIZE);

//==================================================================================================
// MADT Local X2APIC Entry
//==================================================================================================

///
/// # Description
///
/// A structure that describes a Local X2APIC entry in the MADT.
///
#[repr(C, packed)]
pub struct MadtEntryLocalX2Apic {
    pub header: MadtEntryHeader,
    pub reserved: u16,
    pub x2apic_id: u32,
    pub flags: u32,
    pub acpi_processor_uid: u32,
}

impl MadtEntryLocalX2Apic {
    /// Size of the Local X2APIC entry.
    const _SIZE: usize = MadtEntryHeader::_SIZE + 14;
}

static_assert_size!(MadtEntryLocalX2Apic, MadtEntryLocalX2Apic::_SIZE);
