// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Exceptions
//==================================================================================================

// Not all functions are used.
#![allow(dead_code)]

//==================================================================================================
// I/O APIC Register Interface
//==================================================================================================

enum Register {
    /// I/O APIC ID Register (RW)
    IoApicId = 0x00,
    /// I/O APIC Version Register (RO)
    IoApicVersion = 0x01,
    /// I/O APIC Arbitration ID Register (RO)
    IoApicArbitrationId = 0x02,
    /// I/O APIC Redirection Table (RW)
    IoApicRedirectionTable = 0x10,
}

//==================================================================================================
// I/O APIC
//==================================================================================================

/// I/O APIC
#[repr(C, packed)]
#[derive(Debug)]
pub struct Ioapic {
    /// I/O APIC Register Select (RW).
    ioregsel: *mut u32,
    iowin: *mut u32,
}

impl Ioapic {
    pub fn new(addr: usize) -> Self {
        Self {
            ioregsel: addr as *mut u32,
            iowin: (addr + 0x10) as *mut u32,
        }
    }

    /// Reads a register.
    fn read(&mut self, reg: u32) -> u32 {
        unsafe {
            // Select register.
            self.ioregsel.write_volatile(reg);
            // Read data.
            self.iowin.read_volatile()
        }
    }

    /// Writes to a register.
    fn write(&mut self, reg: u32, data: u32) {
        unsafe {
            // Select register.
            self.ioregsel.write_volatile(reg);
            // Write data.
            self.iowin.write_volatile(data);
        }
    }
}

//==================================================================================================
// I/O APIC ID Register
//==================================================================================================

/// I/O APIC ID Register
pub struct IoapicId;

impl IoapicId {
    const IOAPICID_RESERVED_0_SHIFT: u32 = 0;
    const IOAPICID_SHIFT: u32 = 24;
    const IOAPICID_RESERVED_1_SHIFT: u32 = 28;
    const IOAPICID_RESERVED_0_MASK: u32 = 0xffffff << Self::IOAPICID_RESERVED_0_SHIFT;
    const IOAPICID_ID_MASK: u32 = 0x0F << Self::IOAPICID_SHIFT;
    const IOAPICID_RESERVED_1_MASK: u32 = 0xf << Self::IOAPICID_RESERVED_1_SHIFT;

    // Returns the IOAPIC identification.
    pub fn id(ioapic: &mut Ioapic) -> u8 {
        ((ioapic.read(Register::IoApicId as u32) & Self::IOAPICID_ID_MASK) >> Self::IOAPICID_SHIFT)
            as u8
    }

    /// Reads the I/O APIC ID register.
    fn read(ioapic: &mut Ioapic) -> u32 {
        ioapic.read(Register::IoApicId as u32)
    }
}

//==================================================================================================
// I/O APIC Version Register
//==================================================================================================

/// I/O APIC Version Register
pub struct IoapicVersion;

impl IoapicVersion {
    const IOAPICVER_VERSION_SHIFT: u32 = 0;
    const IOAPICVER_RESERVED_0_SHIFT: u32 = 8;
    const IOAPICVER_MAXREDIR_SHIFT: u32 = 16;
    const IOAPICVER_RESERVED_1_SHIFT: u32 = 24;
    const IOAPICVER_VERSION_MASK: u32 = 0xFF << Self::IOAPICVER_VERSION_SHIFT;
    const IOAPICVER_RESERVED_0_MASK: u32 = 0xFF << Self::IOAPICVER_RESERVED_0_SHIFT;
    const IOAPICVER_MAXREDIR_MASK: u32 = 0xFF << Self::IOAPICVER_MAXREDIR_SHIFT;
    const IOAPICVER_RESERVED_1_MASK: u32 = 0xFF << Self::IOAPICVER_RESERVED_1_SHIFT;

    /// Returns the I/O APIC version.
    pub fn version(ioapic: &mut Ioapic) -> u32 {
        (ioapic.read(Register::IoApicVersion as u32) & Self::IOAPICVER_VERSION_MASK)
            >> Self::IOAPICVER_VERSION_SHIFT
    }

    /// Returns the number of redirection entries.
    pub fn maxredirect(ioapic: &mut Ioapic) -> u8 {
        ((ioapic.read(Register::IoApicVersion as u32) & Self::IOAPICVER_MAXREDIR_MASK)
            >> Self::IOAPICVER_MAXREDIR_SHIFT) as u8
    }

    /// Reads the I/O APIC version register.
    fn read(ioapic: &mut Ioapic) -> u32 {
        ioapic.read(Register::IoApicVersion as u32)
    }
}

//==================================================================================================
// I/O APIC Arbitration ID Register
//==================================================================================================

/// I/O APIC Arbitration ID Register
pub struct IoapicArbitrationId;

impl IoapicArbitrationId {
    const IOAPICARB_RESERVED_0_SHIFT: u32 = 0;
    const IOAPICARB_ID_SHIFT: u32 = 24;
    const IOAPICARB_RESERVED_1_SHIFT: u32 = 28;
    const IOAPICARB_RESERVED_0_MASK: u32 = 0xFF << Self::IOAPICARB_RESERVED_0_SHIFT;
    const IOAPICARB_ID_MASK: u32 = 0xFF << Self::IOAPICARB_ID_SHIFT;
    const IOAPICARB_RESERVED_1_MASK: u32 = 0xF << Self::IOAPICARB_RESERVED_1_SHIFT;
}

//==================================================================================================
// I/O APIC Redirection Table
//==================================================================================================

/// I/O APIC Redirection Table (low)
pub struct IoapicRedirectionTableLow;

impl IoapicRedirectionTableLow {
    const IOREDTBL_INTVEC_SHIFT: u32 = 0;
    const IOREDTBL_DELIVMODE_SHIFT: u32 = 8;
    const IOREDTBL_DESTMOD_SHIFT: u32 = 11;
    const IOREDTBL_DELIVS_SHIFT: u32 = 12;
    const IOREDTBL_INTPOL_SHIFT: u32 = 13;
    const IOREDTBL_RIRR_SHIFT: u32 = 14;
    const IOREDTBL_TRIGGER_SHIFT: u32 = 15;
    const IOREDTBL_INTMASK_SHIFT: u32 = 16;
    const IOREDTBL_INTVEC_MASK: u32 = 0xFF << Self::IOREDTBL_INTVEC_SHIFT;
    const IOREDTBL_DELIVMODE_MASK: u32 = 0x7 << Self::IOREDTBL_DELIVMODE_SHIFT;
    const IOREDTBL_DESTMOD_MASK: u32 = 0x1 << Self::IOREDTBL_DESTMOD_SHIFT;
    const IOREDTBL_DELIVS_MASK: u32 = 0x1 << Self::IOREDTBL_DELIVS_SHIFT;
    const IOREDTBL_INTPOL_MASK: u32 = 0x1 << Self::IOREDTBL_INTPOL_SHIFT;
    const IOREDTBL_RIRR_MASK: u32 = 0x1 << Self::IOREDTBL_RIRR_SHIFT;
    const IOREDTBL_TRIGGER_MASK: u32 = 0x1 << Self::IOREDTBL_TRIGGER_SHIFT;
    pub const IOREDTBL_INTMASK_MASK: u32 = 0x1 << Self::IOREDTBL_INTMASK_SHIFT;

    /// Writes to a redirection table entry.
    fn write(ioapic: &mut Ioapic, index: u32, data: u32) {
        ioapic.write(Register::IoApicRedirectionTable as u32 + 2 * index, data);
    }
}

/// I/O APIC Redirection Table (high)
pub struct IoapicRedirectionTableHigh;

impl IoapicRedirectionTableHigh {
    pub const IOREDTBL_DEST_SHIFT: u32 = 24;
    const IOREDTBL_DEST_MASK: u32 = 0xFF << Self::IOREDTBL_DEST_SHIFT;

    /// Writes to a redirection table entry.
    fn write(ioapic: &mut Ioapic, index: u32, data: u32) {
        ioapic.write(Register::IoApicRedirectionTable as u32 + 2 * index + 1, data);
    }
}

/// I/O APIC Redirection Table register.
pub struct IoapicRedirectionTable;

impl IoapicRedirectionTable {
    /// Writes to a redirection table entry.
    pub fn write(ioapic: &mut Ioapic, index: u32, high: u32, low: u32) {
        IoapicRedirectionTableLow::write(ioapic, index, low);
        IoapicRedirectionTableHigh::write(ioapic, index, high);
    }
}
