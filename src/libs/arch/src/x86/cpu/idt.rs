// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::x86::cpu::ring::PrivilegeLevel;

//==================================================================================================
// Gate Type
//==================================================================================================

/// Mask for extracting the gate type from the lower nibble of an IDT flags byte.
const GATE_TYPE_MASK: u8 = 0x0F;
/// Gate type value for a 32-bit task gate.
const GATE_TYPE_TASK32: u8 = 0x5;
/// Gate type value for a 16-bit interrupt gate.
const GATE_TYPE_INT16: u8 = 0x6;
/// Gate type value for a 16-bit trap gate.
const GATE_TYPE_TRAP16: u8 = 0x7;
/// Gate type value for a 32-bit interrupt gate.
const GATE_TYPE_INT32: u8 = 0xe;
/// Gate type value for a 32-bit trap gate.
const GATE_TYPE_TRAP32: u8 = 0xf;

#[repr(u8)]
pub enum GateType {
    Task32 = GATE_TYPE_TASK32, // 32-bit task gate.
    Int16 = GATE_TYPE_INT16,   // 16-bit interrupt gate.
    Trap16 = GATE_TYPE_TRAP16, // 16-bit trap gate.
    Int32 = GATE_TYPE_INT32,   // 32-bit interrupt gate.
    Trap32 = GATE_TYPE_TRAP32, // 32-bit trap gate.
}

impl GateType {
    /// Decodes the gate type from the lower nibble of an IDT flags byte.
    ///
    /// Returns `Some(GateType)` if the nibble matches a known gate type, `None` otherwise.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value & GATE_TYPE_MASK {
            GATE_TYPE_TASK32 => Some(Self::Task32),
            GATE_TYPE_INT16 => Some(Self::Int16),
            GATE_TYPE_TRAP16 => Some(Self::Trap16),
            GATE_TYPE_INT32 => Some(Self::Int32),
            GATE_TYPE_TRAP32 => Some(Self::Trap32),
            _ => None,
        }
    }
}

impl core::fmt::Debug for GateType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", gate_type_str(self))
    }
}

/// Returns a static string label for a [`GateType`] variant.
const fn gate_type_str(gate: &GateType) -> &'static str {
    match gate {
        GateType::Task32 => "task32",
        GateType::Int16 => "int16",
        GateType::Trap16 => "trap16",
        GateType::Int32 => "int32",
        GateType::Trap32 => "trap32",
    }
}

//==================================================================================================
// Present Bit
//==================================================================================================

/// Bit position of the present flag in the IDT flags byte.
const PRESENT_BIT_SHIFT: u8 = 7;

#[repr(u8)]
pub enum PresentBit {
    NotPresent = 0 << PRESENT_BIT_SHIFT,
    Present = 1 << PRESENT_BIT_SHIFT,
}

//==================================================================================================
// Descriptor Privilege Level
//==================================================================================================

/// Bit position of the descriptor privilege level field in the IDT flags byte.
const DPL_SHIFT: u8 = 5;

#[repr(u8)]
pub enum DescriptorPrivilegeLevel {
    Ring0 = (PrivilegeLevel::Ring0 as u8) << DPL_SHIFT,
    Ring1 = (PrivilegeLevel::Ring1 as u8) << DPL_SHIFT,
    Ring2 = (PrivilegeLevel::Ring2 as u8) << DPL_SHIFT,
    Ring3 = (PrivilegeLevel::Ring3 as u8) << DPL_SHIFT,
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

/// Bit position of the upper 16 bits of a 32-bit handler address.
const HANDLER_HIGH_SHIFT: u32 = 16;
/// Bit position of the middle 16 bits of a 64-bit handler address (same value
/// as [`HANDLER_HIGH_SHIFT`] because bits 16..31 sit at the same offset in both modes).
const HANDLER_MID_SHIFT: u32 = 16;
/// Bit position of the upper 32 bits of a 64-bit handler address.
const HANDLER_HIGH64_SHIFT: u32 = 32;

/// Interrupt descriptor table entry (IDTE).
#[repr(C, align(8))]
pub struct Idte {
    handler_low: u16,  // Handler low.
    selector: u16,     // GDT selector.
    zero: u8,          // Always zero.
    flags: u8,         // Gate type and flags.
    handler_high: u16, // Handler high.
}

// `Idte` must be 8 bytes long. This must match the hardware specification.
::static_assert::assert_eq_size!(Idte, 8);

impl Idte {
    /// Creates a new IDT entry.
    pub fn new(handler: u32, selector: u16, flags: Flags) -> Self {
        let handler_low = handler as u16;
        let handler_high = (handler >> HANDLER_HIGH_SHIFT) as u16;

        Self {
            handler_low,
            selector,
            zero: 0,
            flags: flags.into(),
            handler_high,
        }
    }

    /// Returns the full 32-bit handler address reconstructed from the split fields.
    pub fn handler(&self) -> u32 {
        (self.handler_low as u32) | ((self.handler_high as u32) << HANDLER_HIGH_SHIFT)
    }

    /// Returns the decoded gate type from the flags byte, or `None` if unrecognized.
    pub fn gate_type(&self) -> Option<GateType> {
        GateType::from_u8(self.flags)
    }
}

impl core::fmt::Debug for Idte {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let handler: u32 = self.handler();
        let gate_str: &str = self
            .gate_type()
            .as_ref()
            .map_or("unknown", |g| gate_type_str(g));
        write!(
            f,
            "Idte {{ handler={handler:#010x}, selector={:#06x}, flags={:#04x}, {gate_str} }}",
            self.selector, self.flags
        )
    }
}

//==================================================================================================
// Long-Mode Interrupt Descriptor Table Entry
//==================================================================================================

/// Long-mode (64-bit) interrupt descriptor table entry.
///
/// In long mode, each IDT entry is 16 bytes wide and holds a 64-bit handler address.
#[repr(C, packed)]
pub struct Idte64 {
    /// Lower 16 bits of the handler address.
    handler_low: u16,
    /// GDT selector.
    selector: u16,
    /// Interrupt Stack Table index (bits 0–2) and reserved zero bits.
    interrupt_stack_table: u8,
    /// Gate type, DPL, and present bit.
    flags: u8,
    /// Middle 16 bits of the handler address.
    handler_mid: u16,
    /// Upper 32 bits of the handler address.
    handler_high: u32,
    /// Reserved (must be zero).
    reserved: u32,
}

// `Idte64` must be 16 bytes long.
::static_assert::assert_eq_size!(Idte64, 16);

impl Idte64 {
    /// Returns the full 64-bit handler address reconstructed from the split fields.
    pub fn handler(&self) -> u64 {
        // Safety: `addr_of!` + `read_unaligned` avoids creating intermediate
        // references to packed fields, which would be undefined behavior.
        let low: u16 = unsafe { core::ptr::addr_of!(self.handler_low).read_unaligned() };
        let mid: u16 = unsafe { core::ptr::addr_of!(self.handler_mid).read_unaligned() };
        let high: u32 = unsafe { core::ptr::addr_of!(self.handler_high).read_unaligned() };
        (low as u64) | ((mid as u64) << HANDLER_MID_SHIFT) | ((high as u64) << HANDLER_HIGH64_SHIFT)
    }

    /// Returns the flags byte.
    pub fn flags(&self) -> u8 {
        // Safety: same rationale as [`Self::handler`].
        unsafe { core::ptr::addr_of!(self.flags).read_unaligned() }
    }

    /// Returns the GDT selector.
    pub fn selector(&self) -> u16 {
        // Safety: same rationale as [`Self::handler`].
        unsafe { core::ptr::addr_of!(self.selector).read_unaligned() }
    }

    /// Returns the decoded gate type from the flags byte, or `None` if unrecognized.
    pub fn gate_type(&self) -> Option<GateType> {
        GateType::from_u8(self.flags())
    }
}

impl core::fmt::Debug for Idte64 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let handler: u64 = self.handler();
        let flags: u8 = self.flags();
        let selector: u16 = self.selector();
        let gate_str: &str = self
            .gate_type()
            .as_ref()
            .map_or("unknown", |g| gate_type_str(g));
        write!(
            f,
            "Idte64 {{ handler={handler:#018x}, selector={selector:#06x}, flags={flags:#04x}, \
             {gate_str} }}"
        )
    }
}
