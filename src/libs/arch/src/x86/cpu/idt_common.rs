// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::ring::PrivilegeLevel;

//==================================================================================================
// Gate Type
//==================================================================================================

/// Mask for extracting the gate type from the lower nibble of an IDT flags byte.
pub(crate) const GATE_TYPE_MASK: u8 = 0x0F;
/// Gate type value for a 32-bit task gate.
pub(crate) const GATE_TYPE_TASK32: u8 = 0x5;
/// Gate type value for a 16-bit interrupt gate.
pub(crate) const GATE_TYPE_INT16: u8 = 0x6;
/// Gate type value for a 16-bit trap gate.
pub(crate) const GATE_TYPE_TRAP16: u8 = 0x7;
/// Gate type value for a 32-bit interrupt gate.
pub(crate) const GATE_TYPE_INT32: u8 = 0xe;
/// Gate type value for a 32-bit trap gate.
pub(crate) const GATE_TYPE_TRAP32: u8 = 0xf;

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
pub(crate) const fn gate_type_str(gate: &GateType) -> &'static str {
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
pub(crate) const PRESENT_BIT_SHIFT: u8 = 7;

#[repr(u8)]
pub enum PresentBit {
    NotPresent = 0 << PRESENT_BIT_SHIFT,
    Present = 1 << PRESENT_BIT_SHIFT,
}

//==================================================================================================
// Descriptor Privilege Level
//==================================================================================================

/// Bit position of the descriptor privilege level field in the IDT flags byte.
pub(crate) const DPL_SHIFT: u8 = 5;

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
