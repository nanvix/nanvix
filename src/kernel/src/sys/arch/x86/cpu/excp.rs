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
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
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

impl From<u32> for Exception {
    fn from(value: u32) -> Self {
        match value {
            0 => Exception::DivisionByZero,
            1 => Exception::Debug,
            2 => Exception::NonMaskableInterrupt,
            3 => Exception::Breakpoint,
            4 => Exception::Overflow,
            5 => Exception::BoundsCheck,
            6 => Exception::InvalidOpcode,
            7 => Exception::CoprocessorNotAvailable,
            8 => Exception::DoubleFault,
            9 => Exception::CoprocessorSegmentOverrun,
            10 => Exception::InvalidTaskStateSegment,
            11 => Exception::SegmentNotPresent,
            12 => Exception::StackSegmentFault,
            13 => Exception::GeneralProtectionFault,
            14 => Exception::PageFault,
            15 => Exception::Reserved,
            16 => Exception::FloatingPoint,
            17 => Exception::AlignmentCheck,
            18 => Exception::MachineCheck,
            19 => Exception::SmidUnit,
            20 => Exception::Virtualization,
            30 => Exception::Security,
            _ => panic!("invalid exception"),
        }
    }
}
