// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Re-exports
//==================================================================================================

pub use crate::x86::cpu::idt_common::{
    DescriptorPrivilegeLevel,
    Flags,
    GateType,
    PresentBit,
};

//==================================================================================================
// Interrupt Descriptor Table Entry (64-bit)
//==================================================================================================

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

// `Idte` must be 16 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Idte, 16);

/// Bit position of the middle 16 bits of a 64-bit handler address.
const HANDLER_MID_SHIFT: u32 = 16;
/// Bit position of the upper 32 bits of a 64-bit handler address.
const HANDLER_HIGH_SHIFT: u32 = 32;

impl Idte {
    /// Creates a new IDT entry.
    pub fn new(handler: u64, selector: u16, flags: Flags) -> Self {
        let handler_low = handler as u16;
        let handler_mid = (handler >> HANDLER_MID_SHIFT) as u16;
        let handler_high = (handler >> HANDLER_HIGH_SHIFT) as u32;

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
