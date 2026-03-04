// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::x86_64::cpu::ring::PrivilegeLevel;

//==================================================================================================
// Gate Type
//==================================================================================================

#[repr(u8)]
pub enum GateType {
    Int64 = 0xe,  // 64-bit interrupt gate.
    Trap64 = 0xf, // 64-bit trap gate.
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
// Interrupt Descriptor Table Entry (64-bit)
//==================================================================================================

/// Interrupt descriptor table entry (IDTE) for x86_64.
/// In long mode, each IDT entry is 16 bytes with a 64-bit handler address.
#[repr(C, align(16))]
pub struct Idte {
    /// Bits 15:0 of the handler address.
    pub handler_low: u16,
    /// GDT code segment selector.
    pub selector: u16,
    /// Interrupt Stack Table index (bits 2:0), rest must be zero.
    pub ist: u8,
    /// Gate type, DPL, and present flag.
    pub flags: u8,
    /// Bits 31:16 of the handler address.
    pub handler_mid: u16,
    /// Bits 63:32 of the handler address.
    pub handler_high: u32,
    /// Reserved, must be zero.
    pub reserved: u32,
}

// `Idte` must be 16 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Idte, 16);

impl Idte {
    /// Creates a new 64-bit IDT entry.
    ///
    /// # Parameters
    ///
    /// - `handler`: 64-bit address of the interrupt handler.
    /// - `selector`: GDT code segment selector.
    /// - `ist`: Interrupt Stack Table index (0 = no IST, 1-7 = IST entry).
    /// - `flags`: Gate type, DPL, and present flag.
    pub fn new(handler: u64, selector: u16, ist: u8, flags: Flags) -> Self {
        let handler_low = handler as u16;
        let handler_mid = (handler >> 16) as u16;
        let handler_high = (handler >> 32) as u32;

        Self {
            handler_low,
            selector,
            ist: ist & 0x07,
            flags: flags.into(),
            handler_mid,
            handler_high,
            reserved: 0,
        }
    }
}
