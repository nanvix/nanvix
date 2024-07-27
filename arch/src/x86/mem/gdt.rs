// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::x86::cpu::ring::PrivilegeLevel;

//==================================================================================================
// Global Descriptor Table Entry (GDTE)
//==================================================================================================

/// Global descriptor table entry (GDTE).
#[derive(Default)]
#[repr(C, align(8))]
pub struct Gdte {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    flags_limit: u8,
    base_high: u8,
}

// `Gdte` must be 8 bytes long. This must match the hardware specification.
static_assert_size!(Gdte, 8);

// `Gdte` must be aligned to 8 bytes. This must match the hardware specification.
static_assert_alignment!(Gdte, 8);

impl Gdte {
    /// Creates a new GDT entry.
    pub fn new(base: u32, limit: u32, access: GdteAccessByte, flags: GdteFlags) -> Self {
        Self {
            base_low: (base & 0xffff) as u16,
            base_middle: ((base >> 16) & 0xff) as u8,
            base_high: ((base >> 24) & 0xff) as u8,
            limit_low: (limit & 0xffff) as u16,
            flags_limit: (((limit >> 16) & 0x0f) as u8) | (((Into::<u8>::into(flags)) & 0x0f) << 4),
            access: Into::<u8>::into(access),
        }
    }
}

impl core::fmt::Debug for Gdte {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let base: u32 =
            (self.base_high as u32) << 24 | (self.base_middle as u32) << 16 | self.base_low as u32;
        let limit: u32 = (self.flags_limit as u32 & 0x0f) << 16 | self.limit_low as u32;
        let access: u8 = self.access;
        let flags: u8 = (self.flags_limit & 0xf0) >> 4;
        write!(
            f,
            "Gdte {{ base={:#010x}, limit={:#07x}, flags={:#03x}, access={:#04x} }}",
            base, limit, flags, access
        )
    }
}
//==================================================================================================
// Flags
//==================================================================================================

/// Flags for a GDTE.
pub struct GdteFlags {
    granularity: GdteGranularity,
    protected_mode: GdteProtectedMode,
    long_mode: GdteLongMode,
}

impl GdteFlags {
    /// Creates a new set of flags for a GDTE.
    pub fn new(
        granularity: GdteGranularity,
        protected_mode: GdteProtectedMode,
        long_mode: GdteLongMode,
    ) -> Self {
        Self {
            granularity,
            protected_mode,
            long_mode,
        }
    }
}

impl From<GdteFlags> for u8 {
    fn from(val: GdteFlags) -> Self {
        (val.granularity as u8) | (val.protected_mode as u8) | (val.long_mode as u8)
    }
}

/// Granularity flag for a GDTE.
#[derive(Debug)]
#[repr(u8)]
pub enum GdteGranularity {
    ByteGranularity = (0 << 3),
    PageGranularity = (1 << 3),
}

/// Protected mode flag for a GDTE.
#[derive(Debug)]
#[repr(u8)]
pub enum GdteProtectedMode {
    ProtectedMode16 = (0 << 2),
    ProtectedMode32 = (1 << 2),
}

/// Long mode flag for a GDTE.
#[derive(Debug)]
#[repr(u8)]
pub enum GdteLongMode {
    CompatibilityMode = (0 << 1),
    LongMode = (1 << 1),
}

/// Present flag in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessAccessed {
    NotAccessed = 0,
    Accessed = (1 << 0),
}

/// Read flag for code segments in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessReadable {
    NonReadable = (0 << 1),
    Readable = (1 << 1),
}

/// Write flag for data segments in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessWritable {
    Readonly = (0 << 1),
    ReadWrite = (1 << 1),
}

/// Read/Write flag in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessReadWrite {
    CodeSegment(AccessReadable),
    DataSegment(AccessWritable),
}

/// Direction flag in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessDirection {
    GrowsUp = (0 << 2),
    GrowsDown = (1 << 2),
}

/// Conforming flag in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessConforming {
    NonConforming = (0 << 2),
    Conforming = (1 << 2),
}

/// Direction/Conforming flag in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessDirectionConforming {
    Direction(AccessDirection),
    Conforming(AccessConforming),
}

/// Code/Data flag in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessExecutable {
    Data = (0 << 3),
    Code = (1 << 3),
}

/// Descriptor type flag in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessDescriptorType {
    System = (0 << 4),
    CodeData = (1 << 4),
}

/// Descriptor privilege level flag in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum DescriptorPrivilegeLegel {
    Ring0 = (PrivilegeLevel::Ring0 as u8) << 5,
    Ring1 = (PrivilegeLevel::Ring1 as u8) << 5,
    Ring2 = (PrivilegeLevel::Ring2 as u8) << 5,
    Ring3 = (PrivilegeLevel::Ring3 as u8) << 5,
}

/// Present flag in access byte.
#[derive(Debug)]
#[repr(u8)]
pub enum AccessPresent {
    NotPresent = (0 << 7),
    Present = (1 << 7),
}

//==================================================================================================
// Access Byte
//==================================================================================================

/// Access byte for a GDTE.
pub struct GdteAccessByte {
    accessed: AccessAccessed,
    read_write: AccessReadWrite,
    direction_conforming: AccessDirectionConforming,
    executable: AccessExecutable,
    descriptor_type: AccessDescriptorType,
    dpl: DescriptorPrivilegeLegel,
    present: AccessPresent,
}

impl GdteAccessByte {
    /// Creates a new access byte.
    pub fn new(
        accessed: AccessAccessed,
        read_write: AccessReadWrite,
        direction_conforming: AccessDirectionConforming,
        executable: AccessExecutable,
        descriptor_type: AccessDescriptorType,
        dpl: DescriptorPrivilegeLegel,
        present: AccessPresent,
    ) -> Self {
        Self {
            accessed,
            read_write,
            direction_conforming,
            executable,
            descriptor_type,
            dpl,
            present,
        }
    }
}

impl From<GdteAccessByte> for u8 {
    fn from(val: GdteAccessByte) -> Self {
        (val.accessed as u8)
            | match val.read_write {
                AccessReadWrite::CodeSegment(readable) => readable as u8,
                AccessReadWrite::DataSegment(writable) => writable as u8,
            }
            | match val.direction_conforming {
                AccessDirectionConforming::Direction(direction) => direction as u8,
                AccessDirectionConforming::Conforming(conforming) => conforming as u8,
            }
            | (val.executable as u8)
            | (val.descriptor_type as u8)
            | (val.dpl as u8)
            | (val.present as u8)
    }
}
