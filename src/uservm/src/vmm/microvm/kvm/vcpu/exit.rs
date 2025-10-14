// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::kvm::pmio::PmioAccess;

//==================================================================================================
// Structures
//==================================================================================================

/// Virtual processor exit reasons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VirtualProcessorExitReason {
    /// Port-mapped I/O access.
    PmioAccess(PmioAccess),
    /// Halt virtual processor.
    Halt,
    /// Interrupted.
    Interrupted,
    /// Unknown.
    Unknown,
}

/// Virtual processor exit contexts.
pub enum VirtualProcessorExitContext {
    /// Port-mapped I/O.
    Pmio(PmioAccess),
    /// Halt virtual processor.
    Halt,
    /// Interrupt virtual processor.
    Interrupted,
    /// Unknown.
    Unknown,
}

/// Borrowed view of the virtual processor exit reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualProcessorExitReasonRef<'a> {
    /// Port-mapped I/O access.
    PmioAccess(&'a PmioAccess),
    /// Halt virtual processor.
    Halt,
    /// Interrupted.
    Interrupted,
    /// Unknown.
    Unknown,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl VirtualProcessorExitContext {
    ///
    /// # Description
    ///
    /// Gets the reason for a virtual processor exit.
    ///
    /// # Returns
    ///
    /// The reason for the virtual processor exit.
    ///
    pub fn reason(&self) -> VirtualProcessorExitReason {
        self.reason_ref().into()
    }

    /// Returns the virtual processor exit reason without cloning.
    ///
    /// # Description
    ///
    /// Provides a borrowed view of the exit reason to avoid cloning payloads.
    pub fn reason_ref(&self) -> VirtualProcessorExitReasonRef<'_> {
        match self {
            VirtualProcessorExitContext::Pmio(access) => {
                VirtualProcessorExitReasonRef::PmioAccess(access)
            },
            VirtualProcessorExitContext::Halt => VirtualProcessorExitReasonRef::Halt,
            VirtualProcessorExitContext::Interrupted => VirtualProcessorExitReasonRef::Interrupted,
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
            VirtualProcessorExitReasonRef::Halt => VirtualProcessorExitReason::Halt,
            VirtualProcessorExitReasonRef::Interrupted => VirtualProcessorExitReason::Interrupted,
            VirtualProcessorExitReasonRef::Unknown => VirtualProcessorExitReason::Unknown,
        }
    }
}
