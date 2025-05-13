// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use sys::mm::Alignment;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Log2 PAGE_SIZE
///
pub const PAGE_SHIFT: usize = 12;

///
/// # Description
///
/// Number of bytes in a page.
///
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

///
/// # Description
///
/// Mask for page offset.
///
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

///
/// # Description
///
/// Log2 [`PGTAB_SIZE`].
///
pub const PGTAB_SHIFT: usize = 22;

///
/// # Description
///
/// Number of bytes in a page table.
///
pub const PGTAB_SIZE: usize = 1 << PGTAB_SHIFT;

///
/// # Description
///
/// Mask for page table offset.
///
pub const PGTAB_MASK: usize = !(PGTAB_SIZE - 1);

///
/// # Description
///
/// Maximum addressable memory.
///
pub const MAX_ADDRESS: usize = usize::MAX;

///
/// # Description
///
/// Alias for `PAGE_SHIT`.
///
pub const FRAME_SHIFT: usize = PAGE_SHIFT;

///
/// # Description
///
/// Alias for `PAGE_SIZE`.
///
pub const FRAME_SIZE: usize = PAGE_SIZE;

///
/// # Description
///
/// Alignment for a page.
///
pub const PAGE_ALIGNMENT: Alignment = Alignment::Align4096;

///
/// # Description
///
/// Alignment for a page table.
///
pub const PGTAB_ALIGNMENT: Alignment = Alignment::Align4194304;
