// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Virtual processor exit reasons.
///
pub enum VirtualProcessorExitReason {
    /// Port-mapped I/O access.
    PmioAccess,
    /// Halt virtual processor.
    Halt,
    /// Unknown.
    Unknown,
}

///
/// # Description
///
/// Virtual processor exit contexts.
///
pub enum VirtualProcessorExitContext<'a> {
    /// Port-mapped I/O input.
    PmioIn(u16, &'a mut [u8]),
    /// Port-mapped I/O output.
    PmioOut(u16, u32, usize),
    /// Halt virtual processor.
    Halt,
    /// Unknown.
    Unknown,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl VirtualProcessorExitContext<'_> {
    ///
    /// # Description
    ///
    /// Gets the reason for a virtual processor exit.
    ///
    /// # Returns
    ///
    /// The reason for the virtual processor exit.
    ///
    pub fn reason(&self) -> &VirtualProcessorExitReason {
        match self {
            // Port-mapped I/O access.
            VirtualProcessorExitContext::PmioIn(_, _)
            | VirtualProcessorExitContext::PmioOut(_, _, _) => {
                &VirtualProcessorExitReason::PmioAccess
            },
            // Halt virtual processor..
            VirtualProcessorExitContext::Halt => &VirtualProcessorExitReason::Halt,
            // Unknown.
            VirtualProcessorExitContext::Unknown => &VirtualProcessorExitReason::Unknown,
        }
    }
}
