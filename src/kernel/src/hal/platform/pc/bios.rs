// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// BIOS Data Area
///
/// ## References
///
/// https://web.archive.org/web/20120130052813/http://www.nondot.org/sabre/os/files/Booting/BIOS_SEG.txt
///
pub struct BiosDataArea;

//==================================================================================================
// Implementations
//==================================================================================================

impl BiosDataArea {
    /// Base address of the BIOS data area.
    pub const BASE: usize = 0x400;
    /// Offset of the reset vector in the BIOS data area.
    const RESET_VECTOR_OFFSET: usize = 0x67;

    ///
    /// # Description
    ///
    /// Writes to the reset vector in the BIOS data area.
    ///
    /// # Parameters
    ///
    /// - `vector`: New reset vector.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    /// - The BIOS data area is not addressable.
    ///
    pub unsafe fn write_reset_vector(vector: u16) {
        ::core::intrinsics::unaligned_volatile_store(
            (Self::BASE + Self::RESET_VECTOR_OFFSET) as *mut u16,
            vector,
        )
    }
}
