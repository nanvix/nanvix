// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[repr(C, packed)]
/// System Description Table (SDT) header.
#[derive(Debug)]
pub struct AcpiSdtHeader {
    // ASCII string representation of the table ID.
    pub signature: [i8; 4],
    // Length of the table.
    pub length: u32,
    // Revision of the structure.
    pub revision: u8,
    // Checksum of the table.
    pub checksum: u8,
    // ASCII string that identifies the OEM.
    pub oem_id: [i8; 6],
    // ASCII string that identifies the OEM table.
    pub oem_table_id: [i8; 8],
    // OEM revision number.
    pub oem_revision: u32,
    // Vendor ID of the util that created the table.
    pub creator_id: u32,
    // Revision of the util that created the table.
    pub creator_rev: u32,
}

impl AcpiSdtHeader {
    const _SIZE: usize = 36;
}

static_assert_size!(AcpiSdtHeader, AcpiSdtHeader::_SIZE);

#[derive(Debug)]
#[repr(C, packed)]
/// Root System Description Pointer (RSDP).
pub struct Rsdp {
    // ASCII string representation of the signature.
    pub signature: [i8; 8],
    // Checksum of the table.
    pub checksum: u8,
    // ASCII string that identifies the OEM.
    pub oemid: [i8; 6],
    // Revision of the structure.
    pub revision: u8,
    // Address of the RSDT.
    pub rsdt_addr: u32,
}

impl Rsdp {
    const _SIZE: usize = 20;
}

static_assert_size!(Rsdp, Rsdp::_SIZE);
