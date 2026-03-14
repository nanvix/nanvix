// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Enumerations
//==================================================================================================

///
/// # Description
///
/// Exception types.
///
#[repr(u8)]
pub enum Exception {
    /// Divide-by-zero.
    DivisionByZero,
    /// Debug.
    Debug,
    /// Non-maskable interrupt.
    NonMaskableInterrupt,
    /// Breakpoint.
    Breakpoint,
    /// Overflow.
    Overflow,
    /// Bounds check.
    BoundsCheck,
    /// Invalid opcode.
    InvalidOpcode,
    /// Coprocessor not available.
    CoprocessorNotAvailable,
    /// Double fault.
    DoubleFault,
    /// Coprocessor segment overrun.
    CoprocessorSegmentOverrun,
    /// Invalid task state segment.
    InvalidTaskStateSegment,
    /// Segment not present.
    SegmentNotPresent,
    /// Stack segment fault.
    StackSegmentFault,
    /// General protection fault.
    GeneralProtectionFault,
    /// Page fault.
    PageFault,
    /// Reserved.
    Reserved,
    /// Floating-point.
    FloatingPoint,
    /// Alignment check.
    AlignmentCheck,
    /// Machine check.
    MachineCheck,
    /// SMID unit.
    SmidUnit,
    /// Virtualization.
    Virtualization,
    /// Security.
    Security,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl core::fmt::Debug for Exception {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Exception::DivisionByZero => write!(f, "division-by-zero"),
            Exception::Debug => write!(f, "debug"),
            Exception::NonMaskableInterrupt => write!(f, "non-maskable interrupt"),
            Exception::Breakpoint => write!(f, "breakpoint"),
            Exception::Overflow => write!(f, "overflow"),
            Exception::BoundsCheck => write!(f, "bounds check"),
            Exception::InvalidOpcode => write!(f, "invalid opcode"),
            Exception::CoprocessorNotAvailable => write!(f, "coprocessor not available"),
            Exception::DoubleFault => write!(f, "double fault"),
            Exception::CoprocessorSegmentOverrun => write!(f, "coprocessor segment overrun"),
            Exception::InvalidTaskStateSegment => write!(f, "invalid task state segment"),
            Exception::SegmentNotPresent => write!(f, "segment not present"),
            Exception::StackSegmentFault => write!(f, "stack segment fault"),
            Exception::GeneralProtectionFault => write!(f, "general protection fault"),
            Exception::PageFault => write!(f, "page fault"),
            Exception::Reserved => write!(f, "reserved"),
            Exception::FloatingPoint => write!(f, "floating-point"),
            Exception::AlignmentCheck => write!(f, "alignment check"),
            Exception::MachineCheck => write!(f, "machine check"),
            Exception::SmidUnit => write!(f, "smid unit"),
            Exception::Virtualization => write!(f, "virtualization"),
            Exception::Security => write!(f, "security"),
        }
    }
}

impl TryFrom<u32> for Exception {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_from_vector(value as usize).ok_or(value)
    }
}

impl Exception {
    /// Converts an exception vector number into an [`Exception`], returning `None` for
    /// invalid or unrecognized vector numbers.
    pub fn try_from_vector(vector: usize) -> Option<Self> {
        match vector {
            0 => Some(Exception::DivisionByZero),
            1 => Some(Exception::Debug),
            2 => Some(Exception::NonMaskableInterrupt),
            3 => Some(Exception::Breakpoint),
            4 => Some(Exception::Overflow),
            5 => Some(Exception::BoundsCheck),
            6 => Some(Exception::InvalidOpcode),
            7 => Some(Exception::CoprocessorNotAvailable),
            8 => Some(Exception::DoubleFault),
            9 => Some(Exception::CoprocessorSegmentOverrun),
            10 => Some(Exception::InvalidTaskStateSegment),
            11 => Some(Exception::SegmentNotPresent),
            12 => Some(Exception::StackSegmentFault),
            13 => Some(Exception::GeneralProtectionFault),
            14 => Some(Exception::PageFault),
            15 => Some(Exception::Reserved),
            16 => Some(Exception::FloatingPoint),
            17 => Some(Exception::AlignmentCheck),
            18 => Some(Exception::MachineCheck),
            19 => Some(Exception::SmidUnit),
            20 => Some(Exception::Virtualization),
            30 => Some(Exception::Security),
            _ => None,
        }
    }
}
