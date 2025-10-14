// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::kvm::pmio::PmioWidth;

//==================================================================================================
// Structures
//==================================================================================================

/// Port-mapped I/O access details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PmioAccess {
    /// Port-mapped I/O input. Tuple members are `(port, payload_bytes)`.
    PmioIn(u16, Vec<u8>),
    /// Port-mapped I/O output. Tuple members are `(port, value, width)`.
    PmioOut(u16, u32, PmioWidth),
}
