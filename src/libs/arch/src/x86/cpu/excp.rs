// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Wraps the hardware error code pushed by the CPU for exceptions that include one
/// (Double Fault, Invalid TSS, Segment Not Present, Stack Segment Fault,
/// General Protection Fault, Page Fault, Alignment Check, and Security).
///
/// For page faults the individual bits carry specific meaning defined by the x86 ISA.
/// For selector-based exceptions (GP, SS, NP, TSS) the low 16 bits encode the
/// offending segment selector.
///
#[derive(Clone, Copy)]
#[must_use]
pub struct ErrorCode(u32);

impl ErrorCode {
    /// Creates a new [`ErrorCode`] from a raw 32-bit value.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw 32-bit value.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Page-fault bit 0 — *Present*.
    ///
    /// `true` when the fault was caused by a page-level protection violation.
    /// `false` when the fault was caused by a non-present page.
    pub const fn is_present(self) -> bool {
        (self.0 & (1 << 0)) != 0
    }

    /// Page-fault bit 1 — *Write*.
    ///
    /// `true` when the access that caused the fault was a write.
    pub const fn is_write(self) -> bool {
        (self.0 & (1 << 1)) != 0
    }

    /// Page-fault bit 2 — *User*.
    ///
    /// `true` when the fault occurred while the CPU was in user mode (CPL = 3).
    pub const fn is_user(self) -> bool {
        (self.0 & (1 << 2)) != 0
    }

    /// Page-fault bit 3 — *Reserved-bit violation*.
    ///
    /// `true` when a reserved bit was set in a page-structure entry.
    pub const fn is_reserved_bit_violation(self) -> bool {
        (self.0 & (1 << 3)) != 0
    }

    /// Page-fault bit 4 — *Instruction fetch*.
    ///
    /// `true` when the fault was caused by an instruction fetch (requires NX support).
    pub const fn is_instruction_fetch(self) -> bool {
        (self.0 & (1 << 4)) != 0
    }
}

impl core::fmt::Debug for ErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "ErrorCode({:#x} [P={}, W={}, U={}, RSVD={}, I/D={}])",
            self.0,
            self.is_present() as u8,
            self.is_write() as u8,
            self.is_user() as u8,
            self.is_reserved_bit_violation() as u8,
            self.is_instruction_fetch() as u8,
        )
    }
}

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
    /// Maximum number of bytes the CPU pushes onto the stack when an exception happens (with a
    /// privilege-level change): EIP (4) + CS (4) + EFLAGS (4) + ESP (4) + SS (4) + error code (4) =
    /// 24 bytes.
    pub const CONTEXT_HW_SIZE: usize = 24;

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
