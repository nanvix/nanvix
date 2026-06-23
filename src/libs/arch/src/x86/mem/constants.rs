// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use sys::mm::Alignment;
use vstd::prelude::*;

#[allow(unused_imports)]
use ::vstd::prelude::*;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Number of bytes in a word.
///
pub const WORD_SIZE: usize = ::core::mem::size_of::<u32>();

///
/// # Description
///
/// Alignment for a word.
///
pub const WORD_ALIGNMENT: Alignment = Alignment::Align4;

///
/// # Description
///
/// Log2 WORD_SIZE
///
pub const WORD_SHIFT: usize = WORD_SIZE.trailing_zeros() as usize;

///
/// # Description
///
/// Log2 PAGE_SIZE
///
#[verus_verify]
pub const PAGE_SHIFT: usize = 12;

///
/// # Description
///
/// Number of bytes in a page.
///
#[verus_verify]
pub const PAGE_SIZE: usize = 4096;
// Compile-time check that the literal matches the shift-based definition.
::static_assert::assert_eq!(PAGE_SIZE == 1 << PAGE_SHIFT);

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
#[verus_verify]
pub const PGTAB_SHIFT: usize = 22;

///
/// # Description
///
/// Number of bytes in a page table.
///
#[verus_verify]
pub const PGTAB_SIZE: usize = 1 << PGTAB_SHIFT;

///
/// # Description
///
/// Number of entries in a page table.
///
#[verus_verify]
pub const PAGE_TABLE_LENGTH: usize = PGTAB_SIZE / PAGE_SIZE;

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
#[verus_verify]
pub const MAX_ADDRESS: usize = usize::MAX;

///
/// # Description
///
/// Alias for `PAGE_SHIT`.
///
#[verus_verify]
pub const FRAME_SHIFT: usize = PAGE_SHIFT;

///
/// # Description
///
/// Alias for `PAGE_SIZE`.
///
#[verus_verify]
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

///
/// # Description
///
/// Stack alignment mandated by the System V ABI.
///
pub const STACK_ALIGNMENT: Alignment = Alignment::Align16;
