// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::x86::cpu::ring::PrivilegeLevel;

//==================================================================================================
// Gate Type
//==================================================================================================

#[repr(u8)]
pub enum GateType {
    Task32 = 0x5, // 32-bit task gate.
    Int16 = 0x6,  // 16-bit interrupt gate.
    Trap16 = 0x7, // 16-bit trap gate.
    Int32 = 0xe,  // 32-bit interrupt gate (also used for 64-bit on x86_64).
    Trap32 = 0xf, // 32-bit trap gate (also used for 64-bit on x86_64).
}

//==================================================================================================
// Present Bit
//==================================================================================================

#[repr(u8)]
pub enum PresentBit {
    NotPresent = 0 << 7,
    Present = 1 << 7,
}

//==================================================================================================
// Descriptor Privilege Level
//==================================================================================================

#[repr(u8)]
pub enum DescriptorPrivilegeLevel {
    Ring0 = (PrivilegeLevel::Ring0 as u8) << 5,
    Ring1 = (PrivilegeLevel::Ring1 as u8) << 5,
    Ring2 = (PrivilegeLevel::Ring2 as u8) << 5,
    Ring3 = (PrivilegeLevel::Ring3 as u8) << 5,
}

//==================================================================================================
// Flags
//==================================================================================================

pub struct Flags {
    present: PresentBit,
    dpl: DescriptorPrivilegeLevel,
    typ: GateType,
}

impl Flags {
    pub fn new(present: PresentBit, dpl: DescriptorPrivilegeLevel, typ: GateType) -> Self {
        Self { present, dpl, typ }
    }
}

impl From<Flags> for u8 {
    fn from(val: Flags) -> Self {
        val.present as u8 | val.dpl as u8 | val.typ as u8
    }
}

//==================================================================================================
// Interrupt Descriptor Table Entry
//==================================================================================================

#[cfg(target_arch = "x86")]
/// Interrupt descriptor table entry (IDTE) for 32-bit x86.
#[repr(C, align(8))]
pub struct Idte {
    pub handler_low: u16,  // Handler low.
    pub selector: u16,     // GDT selector.
    pub zero: u8,          // Always zero.
    pub flags: u8,         // Gate type and flags.
    pub handler_high: u16, // Handler high.
}

#[cfg(target_arch = "x86")]
// `Idte` must be 8 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Idte, 8);

#[cfg(target_arch = "x86")]
impl Idte {
    /// Creates a new IDT entry.
    pub fn new(handler: u32, selector: u16, flags: Flags) -> Self {
        let handler_low = handler as u16;
        let handler_high = (handler >> 16) as u16;

        Self {
            handler_low,
            selector,
            zero: 0,
            flags: flags.into(),
            handler_high,
        }
    }
}

#[cfg(target_arch = "x86_64")]
/// Interrupt descriptor table entry (IDTE) for 64-bit x86_64.
#[repr(C, packed)]
pub struct Idte {
    pub handler_low: u16,  // Handler bits [0:15].
    pub selector: u16,     // GDT selector.
    pub ist: u8,           // IST index (bits [0:2]), rest zero.
    pub flags: u8,         // Gate type and flags.
    pub handler_mid: u16,  // Handler bits [16:31].
    pub handler_high: u32, // Handler bits [32:63].
    pub reserved: u32,     // Must be zero.
}

#[cfg(target_arch = "x86_64")]
// `Idte` must be 16 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Idte, 16);

#[cfg(target_arch = "x86_64")]
impl Idte {
    /// Creates a new IDT entry.
    pub fn new(handler: u64, selector: u16, flags: Flags) -> Self {
        let handler_low = handler as u16;
        let handler_mid = (handler >> 16) as u16;
        let handler_high = (handler >> 32) as u32;

        Self {
            handler_low,
            selector,
            ist: 0,
            flags: flags.into(),
            handler_mid,
            handler_high,
            reserved: 0,
        }
    }
}
