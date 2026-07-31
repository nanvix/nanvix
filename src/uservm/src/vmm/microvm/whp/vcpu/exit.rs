// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Enum-to-usize casts are intentional for PmioWidth conversion.
#![allow(clippy::cast_possible_truncation)]

//==================================================================================================
// Structures
//==================================================================================================

/// Port-mapped I/O transfer widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmioWidth {
    /// Byte-sized (1 byte) access.
    Byte = 1,
    /// Word-sized (2 bytes) access.
    Word = 2,
    /// Doubleword-sized (4 bytes) access.
    Dword = 4,
}

/// Port-mapped I/O access details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PmioAccess {
    /// Port-mapped I/O input. Tuple members are `(port, payload_bytes)`.
    PmioIn(u16, Vec<u8>),
    /// Port-mapped I/O output. Tuple members are `(port, value, width)`.
    PmioOut(u16, u32, PmioWidth),
}

/// Memory-mapped I/O access details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MmioAccess {
    /// Guest physical address that was accessed.
    gpa: u64,
    /// Guest program counter at the trapped instruction.
    pc: u64,
    /// Architecture-specific fault syndrome.
    syndrome: u64,
}

impl MmioAccess {
    /// Creates an MMIO access without architecture-specific exit metadata.
    pub const fn new(gpa: u64) -> Self {
        Self {
            gpa,
            pc: 0,
            syndrome: 0,
        }
    }

    /// Creates an AArch64 MMIO access.
    pub const fn new_aarch64(gpa: u64, pc: u64, syndrome: u64) -> Self {
        Self { gpa, pc, syndrome }
    }

    /// Returns the accessed guest physical address.
    pub const fn gpa(self) -> u64 {
        self.gpa
    }

    /// Returns the guest program counter at the trapped instruction.
    pub const fn pc(self) -> u64 {
        self.pc
    }

    /// Returns the architecture-specific fault syndrome.
    pub const fn syndrome(self) -> u64 {
        self.syndrome
    }
}

/// Guest reset request reported by the hypervisor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetKind {
    /// Power off the virtual machine.
    PowerOff,
    /// Reboot the virtual machine.
    Reboot,
}

/// Virtual processor exit reasons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VirtualProcessorExitReason {
    /// Port-mapped I/O access.
    PmioAccess(PmioAccess),
    /// Memory-mapped I/O access.
    MmioAccess(MmioAccess),
    /// Halt virtual processor.
    Halt,
    /// Guest reset request.
    Reset(ResetKind),
    /// Interrupted.
    Interrupted,
    /// Interrupt window opened (IF transitioned to 1).
    InterruptWindow,
    /// Unknown.
    Unknown,
}

/// Virtual processor exit contexts.
pub enum VirtualProcessorExitContext {
    /// Port-mapped I/O.
    Pmio(PmioAccess),
    /// Memory-mapped I/O.
    Mmio(MmioAccess),
    /// Halt virtual processor.
    Halt,
    /// Guest reset request.
    Reset(ResetKind),
    /// Interrupt virtual processor.
    Interrupted,
    /// Interrupt window opened (IF transitioned to 1).
    InterruptWindow,
    /// Unknown.
    Unknown,
}

/// Borrowed view of the virtual processor exit reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualProcessorExitReasonRef<'a> {
    /// Port-mapped I/O access.
    PmioAccess(&'a PmioAccess),
    /// Memory-mapped I/O access.
    MmioAccess(MmioAccess),
    /// Halt virtual processor.
    Halt,
    /// Guest reset request.
    Reset(ResetKind),
    /// Interrupted.
    Interrupted,
    /// Interrupt window opened (IF transitioned to 1).
    InterruptWindow,
    /// Unknown.
    Unknown,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl From<PmioWidth> for usize {
    fn from(value: PmioWidth) -> Self {
        value as usize
    }
}

impl From<&PmioWidth> for usize {
    fn from(value: &PmioWidth) -> Self {
        *value as usize
    }
}

impl TryFrom<usize> for PmioWidth {
    type Error = usize;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Byte),
            2 => Ok(Self::Word),
            4 => Ok(Self::Dword),
            invalid => Err(invalid),
        }
    }
}

impl VirtualProcessorExitContext {
    ///
    /// # Description
    ///
    /// Gets the reason for a virtual processor exit.
    ///
    pub fn reason(&self) -> VirtualProcessorExitReason {
        self.reason_ref().into()
    }

    /// Returns the virtual processor exit reason without cloning.
    pub fn reason_ref(&self) -> VirtualProcessorExitReasonRef<'_> {
        match self {
            VirtualProcessorExitContext::Pmio(access) => {
                VirtualProcessorExitReasonRef::PmioAccess(access)
            },
            VirtualProcessorExitContext::Mmio(access) => {
                VirtualProcessorExitReasonRef::MmioAccess(*access)
            },
            VirtualProcessorExitContext::Halt => VirtualProcessorExitReasonRef::Halt,
            VirtualProcessorExitContext::Reset(kind) => VirtualProcessorExitReasonRef::Reset(*kind),
            VirtualProcessorExitContext::Interrupted => VirtualProcessorExitReasonRef::Interrupted,
            VirtualProcessorExitContext::InterruptWindow => {
                VirtualProcessorExitReasonRef::InterruptWindow
            },
            VirtualProcessorExitContext::Unknown => VirtualProcessorExitReasonRef::Unknown,
        }
    }
}

impl<'a> From<VirtualProcessorExitReasonRef<'a>> for VirtualProcessorExitReason {
    fn from(value: VirtualProcessorExitReasonRef<'a>) -> Self {
        match value {
            VirtualProcessorExitReasonRef::PmioAccess(access) => {
                VirtualProcessorExitReason::PmioAccess(access.clone())
            },
            VirtualProcessorExitReasonRef::MmioAccess(access) => {
                VirtualProcessorExitReason::MmioAccess(access)
            },
            VirtualProcessorExitReasonRef::Halt => VirtualProcessorExitReason::Halt,
            VirtualProcessorExitReasonRef::Reset(kind) => VirtualProcessorExitReason::Reset(kind),
            VirtualProcessorExitReasonRef::Interrupted => VirtualProcessorExitReason::Interrupted,
            VirtualProcessorExitReasonRef::InterruptWindow => {
                VirtualProcessorExitReason::InterruptWindow
            },
            VirtualProcessorExitReasonRef::Unknown => VirtualProcessorExitReason::Unknown,
        }
    }
}
