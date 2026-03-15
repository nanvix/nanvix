// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::x86::cpu::ring::PrivilegeLevel;

//==================================================================================================
// Base Address Constants
//==================================================================================================

/// Mask for extracting the lower 16 bits of the segment base address.
const BASE_LOW_MASK: u32 = 0xffff;
/// Bit position of the middle 8 bits of the segment base address.
const BASE_MIDDLE_SHIFT: u32 = 16;
/// Mask for extracting the middle 8 bits of the segment base address.
const BASE_MIDDLE_MASK: u32 = 0xff;
/// Bit position of the upper 8 bits of the segment base address.
const BASE_HIGH_SHIFT: u32 = 24;
/// Mask for extracting the upper 8 bits of the segment base address.
const BASE_HIGH_MASK: u32 = 0xff;

//==================================================================================================
// Segment Limit Constants
//==================================================================================================

/// Mask for extracting the lower 16 bits of the segment limit.
const LIMIT_LOW_MASK: u32 = 0xffff;
/// Bit position of the upper 4 bits of the segment limit.
const LIMIT_HIGH_SHIFT: u32 = 16;
/// Mask for extracting the upper 4 bits of the segment limit.
const LIMIT_HIGH_MASK: u32 = 0x0f;

//==================================================================================================
// Flags/Limit Byte Layout Constants
//==================================================================================================

/// Bit position of the flags nibble in the flags/limit byte.
const FLAGS_NIBBLE_SHIFT: u8 = 4;
/// Mask for extracting a 4-bit nibble from the flags/limit byte.
const FLAGS_NIBBLE_MASK: u8 = 0x0f;

//==================================================================================================
// Global Descriptor Table Entry (GDTE)
//==================================================================================================

///
/// # Description
///
/// A type that represents an entry in the Global Descriptor Table (GDT).
///
#[repr(C, align(8))]
pub struct Gdte {
    /// The lower 16 bits of the segment limit.
    limit_low: u16,
    /// The lower 16 bits of the base address.
    base_low: u16,
    /// The middle 8 bits of the base address.
    base_middle: u8,
    /// The access byte for the segment.
    access: u8,
    /// The flags and the upper 8 bits of the segment limit.
    flags_limit: u8,
    /// The upper 8 bits of the base address.
    base_high: u8,
}

// `Gdte` must be 8 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Gdte, 8);

// `Gdte` must be aligned to 8 bytes. This must match the hardware specification.
::static_assert::assert_eq_align!(Gdte, 8);

impl Gdte {
    ///
    /// # Description
    ///
    /// Creates a new GDT entry.
    ///
    /// # Parameters
    ///
    /// - `base`: The base address of the segment.
    /// - `limit`: The limit of the segment.
    /// - `access`: The access byte for the segment.
    /// - `flags`: The flags for the segment.
    ///
    /// # Return Value
    ///
    /// This function returns a new GDT entry.
    ///
    pub const fn new(base: u32, limit: u32, access: GdteAccessByte, flags: GdteFlags) -> Self {
        Self {
            base_low: Self::compute_base_low(base),
            base_high: Self::compute_base_high(base),
            base_middle: Self::compute_base_middle(base),
            limit_low: Self::compute_limit_low(limit),
            flags_limit: (Self::compute_flags_low(flags.into_u8()) << FLAGS_NIBBLE_SHIFT)
                | (Self::compute_limit_high(limit) & FLAGS_NIBBLE_MASK),
            access: access.into_u8(),
        }
    }

    ///
    /// # Description
    ///
    /// Creates a new GDT entry with default values.
    ///
    /// # Return Value
    ///
    /// This function returns a new GDT entry with default values.
    ///
    pub const fn default() -> Self {
        Self {
            base_low: 0,
            base_middle: 0,
            base_high: 0,
            limit_low: 0,
            flags_limit: 0,
            access: 0,
        }
    }

    ///
    /// # Description
    ///
    /// Sets the base address of the target GDT entry.
    ///
    /// # Parameters
    ///
    /// - `base`: The new base address for the target GDT entry.
    ///
    pub fn set_base(&mut self, base: u32) {
        self.base_low = Self::compute_base_low(base);
        self.base_middle = Self::compute_base_middle(base);
        self.base_high = Self::compute_base_high(base);
    }

    ///
    /// # Description
    ///
    /// Gets the base address of the target GDT entry.
    ///
    /// # Return Value
    ///
    /// This function returns the base address of the target GDT entry.
    ///
    pub fn get_base(&self) -> u32 {
        (self.base_high as u32) << BASE_HIGH_SHIFT
            | (self.base_middle as u32) << BASE_MIDDLE_SHIFT
            | (self.base_low as u32)
    }

    /// Returns the raw access byte of the GDT entry.
    pub fn access_byte(&self) -> u8 {
        self.access
    }

    /// Returns the 4-bit flags nibble (granularity, D/B, L, AVL) of the GDT entry.
    pub fn get_flags(&self) -> u8 {
        (self.flags_limit >> FLAGS_NIBBLE_SHIFT) & FLAGS_NIBBLE_MASK
    }

    /// Returns the 20-bit segment limit of the GDT entry.
    pub fn get_limit(&self) -> u32 {
        (self.flags_limit as u32 & LIMIT_HIGH_MASK) << LIMIT_HIGH_SHIFT | self.limit_low as u32
    }

    ///
    /// # Description
    ///
    /// Computes the lower 16 bits of the segment base address.
    ///
    /// # Parameters
    ///
    /// - `base`: The base address to extract bits from.
    ///
    /// # Return Value
    ///
    /// This function returns the lower 16 bits of the segment base address.
    ///
    #[inline(always)]
    const fn compute_base_low(base: u32) -> u16 {
        (base & BASE_LOW_MASK) as u16
    }

    ///
    /// # Description
    ///
    /// Computes the middle 8 bits of the segment base address.
    ///
    /// # Parameters
    ///
    /// - `base`: The base address to extract bits from.
    ///
    /// # Return Value
    ///
    /// This function returns the middle 8 bits of the segment base address.
    ///
    #[inline(always)]
    const fn compute_base_middle(base: u32) -> u8 {
        ((base >> BASE_MIDDLE_SHIFT) & BASE_MIDDLE_MASK) as u8
    }

    ///
    /// # Description
    ///
    /// Computes the upper 8 bits of the segment base address.
    ///
    /// # Parameters
    ///
    /// - `base`: The base address to extract bits from.
    ///
    /// # Return Value
    ///
    /// This function returns the upper 8 bits of the segment base address.
    ///
    #[inline(always)]
    const fn compute_base_high(base: u32) -> u8 {
        ((base >> BASE_HIGH_SHIFT) & BASE_HIGH_MASK) as u8
    }

    ///
    /// # Description
    ///
    /// Computes the lower 16 bits of the segment limit.
    ///
    /// # Parameters
    ///
    /// - `limit`: The segment limit to extract bits from.
    ///
    /// # Return Value
    ///
    /// This function returns the lower 16 bits of the segment limit.
    ///
    #[inline(always)]
    const fn compute_limit_low(limit: u32) -> u16 {
        (limit & LIMIT_LOW_MASK) as u16
    }

    ///
    /// # Description
    ///
    /// Computes the upper 4 bits of the segment limit.
    ///
    /// # Parameters
    ///
    /// - `limit`: The segment limit to extract bits from.
    ///
    /// # Return Value
    ///
    /// This function returns the upper 4 bits of the segment limit.
    ///
    #[inline(always)]
    const fn compute_limit_high(limit: u32) -> u8 {
        ((limit >> LIMIT_HIGH_SHIFT) & LIMIT_HIGH_MASK) as u8
    }

    ///
    /// # Description
    ///
    /// Computes the lower 4 bits of the flags.
    ///
    /// # Parameters
    ///
    /// - `flags`: The flags to extract bits from.
    ///
    /// # Return Value
    ///
    /// This function returns the lower 4 bits of the flags.
    ///
    #[inline(always)]
    const fn compute_flags_low(flags: u8) -> u8 {
        flags & FLAGS_NIBBLE_MASK
    }
}

impl core::fmt::Debug for Gdte {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let base: u32 = self.get_base();
        let limit: u32 = self.get_limit();
        let access_raw: u8 = self.access;
        let access: GdteAccessByte = GdteAccessByte::from_u8(access_raw);
        let flags: u8 = self.get_flags();
        write!(
            f,
            "Gdte {{ base={:#010x}, limit={:#07x}, flags={:#03x}, access={access_raw:#04x} \
             ({access:?}) }}",
            base, limit, flags
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
    pub const fn new(
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

    ///
    /// # Description
    ///
    /// Converts the flags into a `u8`.
    ///
    /// # Return Value
    ///
    /// This function returns the flags as a `u8`.
    ///
    const fn into_u8(&self) -> u8 {
        (self.granularity as u8) | (self.protected_mode as u8) | (self.long_mode as u8)
    }
}

/// Bit position of the granularity flag in the GDT flags nibble.
const GRANULARITY_SHIFT: u8 = 3;
/// Bit position of the protected mode flag in the GDT flags nibble.
const PROTECTED_MODE_SHIFT: u8 = 2;
/// Bit position of the long mode flag in the GDT flags nibble.
const LONG_MODE_SHIFT: u8 = 1;

/// Granularity flag for a GDTE.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum GdteGranularity {
    ByteGranularity = (0 << GRANULARITY_SHIFT),
    PageGranularity = (1 << GRANULARITY_SHIFT),
}

/// Protected mode flag for a GDTE.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum GdteProtectedMode {
    ProtectedMode16 = (0 << PROTECTED_MODE_SHIFT),
    ProtectedMode32 = (1 << PROTECTED_MODE_SHIFT),
}

/// Long mode flag for a GDTE.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum GdteLongMode {
    CompatibilityMode = (0 << LONG_MODE_SHIFT),
    LongMode = (1 << LONG_MODE_SHIFT),
}

//==================================================================================================
// Access Byte Constants
//==================================================================================================

/// Bit position of the read/write flag in the access byte.
const ACCESS_RW_SHIFT: u8 = 1;
/// Bit position of the direction/conforming flag in the access byte.
const ACCESS_DC_SHIFT: u8 = 2;
/// Bit position of the executable flag in the access byte.
const ACCESS_EXECUTABLE_SHIFT: u8 = 3;
/// Bit position of the descriptor type flag in the access byte.
const ACCESS_DESCRIPTOR_TYPE_SHIFT: u8 = 4;
/// Bit position of the descriptor privilege level field in the access byte.
const ACCESS_DPL_SHIFT: u8 = 5;
/// Mask for extracting the 2-bit DPL field from a shifted access byte.
const ACCESS_DPL_MASK: u8 = 0x3;
/// Bit position of the present flag in the access byte.
const ACCESS_PRESENT_SHIFT: u8 = 7;

/// Present flag in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessAccessed {
    NotAccessed = 0,
    Accessed = (1 << 0),
}

/// Read flag for code segments in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessReadable {
    NonReadable = (0 << ACCESS_RW_SHIFT),
    Readable = (1 << ACCESS_RW_SHIFT),
}

/// Write flag for data segments in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessWritable {
    Readonly = (0 << ACCESS_RW_SHIFT),
    ReadWrite = (1 << ACCESS_RW_SHIFT),
}

/// Read/Write flag in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessReadWrite {
    CodeSegment(AccessReadable),
    DataSegment(AccessWritable),
}

/// Direction flag in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessDirection {
    GrowsUp = (0 << ACCESS_DC_SHIFT),
    GrowsDown = (1 << ACCESS_DC_SHIFT),
}

/// Conforming flag in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessConforming {
    NonConforming = (0 << ACCESS_DC_SHIFT),
    Conforming = (1 << ACCESS_DC_SHIFT),
}

/// Direction/Conforming flag in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessDirectionConforming {
    Direction(AccessDirection),
    Conforming(AccessConforming),
}

/// Code/Data flag in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessExecutable {
    Data = (0 << ACCESS_EXECUTABLE_SHIFT),
    Code = (1 << ACCESS_EXECUTABLE_SHIFT),
}

/// Descriptor type flag in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessDescriptorType {
    System = (0 << ACCESS_DESCRIPTOR_TYPE_SHIFT),
    CodeData = (1 << ACCESS_DESCRIPTOR_TYPE_SHIFT),
}

/// Descriptor privilege level flag in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum DescriptorPrivilegeLevel {
    Ring0 = (PrivilegeLevel::Ring0 as u8) << ACCESS_DPL_SHIFT,
    Ring1 = (PrivilegeLevel::Ring1 as u8) << ACCESS_DPL_SHIFT,
    Ring2 = (PrivilegeLevel::Ring2 as u8) << ACCESS_DPL_SHIFT,
    Ring3 = (PrivilegeLevel::Ring3 as u8) << ACCESS_DPL_SHIFT,
}

/// Present flag in access byte.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccessPresent {
    NotPresent = (0 << ACCESS_PRESENT_SHIFT),
    Present = (1 << ACCESS_PRESENT_SHIFT),
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
    dpl: DescriptorPrivilegeLevel,
    present: AccessPresent,
}

impl GdteAccessByte {
    /// Creates a new access byte.
    pub const fn new(
        accessed: AccessAccessed,
        read_write: AccessReadWrite,
        direction_conforming: AccessDirectionConforming,
        executable: AccessExecutable,
        descriptor_type: AccessDescriptorType,
        dpl: DescriptorPrivilegeLevel,
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

    ///
    /// # Description
    ///
    /// Converts the access byte into a `u8`.
    ///
    /// # Return Value
    ///
    /// This function returns the access byte as a `u8`.
    ///
    const fn into_u8(&self) -> u8 {
        (self.accessed as u8)
            | match self.read_write {
                AccessReadWrite::CodeSegment(readable) => readable as u8,
                AccessReadWrite::DataSegment(writable) => writable as u8,
            }
            | match self.direction_conforming {
                AccessDirectionConforming::Direction(direction) => direction as u8,
                AccessDirectionConforming::Conforming(conforming) => conforming as u8,
            }
            | (self.executable as u8)
            | (self.descriptor_type as u8)
            | (self.dpl as u8)
            | (self.present as u8)
    }

    /// Decodes a raw access byte into a [`GdteAccessByte`].
    ///
    /// For system descriptors (bit 4 clear), bits 0–3 encode the system-segment
    /// type rather than the code/data flags.  In that case the returned
    /// `executable`, `read_write`, and `direction_conforming` fields are set to
    /// their default (data/readonly/grows-up) values and should be ignored;
    /// only `descriptor_type`, `dpl`, and `present` are meaningful.
    pub fn from_u8(value: u8) -> Self {
        let descriptor_type: AccessDescriptorType =
            if (value & (AccessDescriptorType::CodeData as u8)) != 0 {
                AccessDescriptorType::CodeData
            } else {
                AccessDescriptorType::System
            };
        let dpl: DescriptorPrivilegeLevel = match (value >> ACCESS_DPL_SHIFT) & ACCESS_DPL_MASK {
            0 => DescriptorPrivilegeLevel::Ring0,
            1 => DescriptorPrivilegeLevel::Ring1,
            2 => DescriptorPrivilegeLevel::Ring2,
            _ => DescriptorPrivilegeLevel::Ring3,
        };
        let present: AccessPresent = if (value & (AccessPresent::Present as u8)) != 0 {
            AccessPresent::Present
        } else {
            AccessPresent::NotPresent
        };

        // For system descriptors, bits 0–3 encode the system-segment type
        // rather than accessed/rw/dc/executable.  Use neutral defaults.
        let (accessed, executable, read_write, direction_conforming) = match descriptor_type {
            AccessDescriptorType::System => (
                AccessAccessed::NotAccessed,
                AccessExecutable::Data,
                AccessReadWrite::DataSegment(AccessWritable::Readonly),
                AccessDirectionConforming::Direction(AccessDirection::GrowsUp),
            ),
            AccessDescriptorType::CodeData => {
                let accessed: AccessAccessed = if (value & (AccessAccessed::Accessed as u8)) != 0 {
                    AccessAccessed::Accessed
                } else {
                    AccessAccessed::NotAccessed
                };
                let executable: AccessExecutable = if (value & (AccessExecutable::Code as u8)) != 0
                {
                    AccessExecutable::Code
                } else {
                    AccessExecutable::Data
                };
                let read_write: AccessReadWrite = match executable {
                    AccessExecutable::Code => {
                        if (value & (AccessReadable::Readable as u8)) != 0 {
                            AccessReadWrite::CodeSegment(AccessReadable::Readable)
                        } else {
                            AccessReadWrite::CodeSegment(AccessReadable::NonReadable)
                        }
                    },
                    AccessExecutable::Data => {
                        if (value & (AccessWritable::ReadWrite as u8)) != 0 {
                            AccessReadWrite::DataSegment(AccessWritable::ReadWrite)
                        } else {
                            AccessReadWrite::DataSegment(AccessWritable::Readonly)
                        }
                    },
                };
                let direction_conforming: AccessDirectionConforming = match executable {
                    AccessExecutable::Code => {
                        if (value & (AccessConforming::Conforming as u8)) != 0 {
                            AccessDirectionConforming::Conforming(AccessConforming::Conforming)
                        } else {
                            AccessDirectionConforming::Conforming(AccessConforming::NonConforming)
                        }
                    },
                    AccessExecutable::Data => {
                        if (value & (AccessDirection::GrowsDown as u8)) != 0 {
                            AccessDirectionConforming::Direction(AccessDirection::GrowsDown)
                        } else {
                            AccessDirectionConforming::Direction(AccessDirection::GrowsUp)
                        }
                    },
                };
                (accessed, executable, read_write, direction_conforming)
            },
        };

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

impl core::fmt::Debug for GdteAccessByte {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let present: &str = match self.present {
            AccessPresent::Present => "present",
            AccessPresent::NotPresent => "not-present",
        };
        let descriptor_type: &str = match self.descriptor_type {
            AccessDescriptorType::CodeData => match self.executable {
                AccessExecutable::Code => "code",
                AccessExecutable::Data => "data",
            },
            AccessDescriptorType::System => "system",
        };
        let dpl: u8 = match self.dpl {
            DescriptorPrivilegeLevel::Ring0 => 0,
            DescriptorPrivilegeLevel::Ring1 => 1,
            DescriptorPrivilegeLevel::Ring2 => 2,
            DescriptorPrivilegeLevel::Ring3 => 3,
        };
        write!(f, "{present}, {descriptor_type}, DPL={dpl}")
    }
}
