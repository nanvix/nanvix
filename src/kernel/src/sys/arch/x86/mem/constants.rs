// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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

pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

pub const PGTAB_SHIFT: usize = 22;

pub const PGTAB_SIZE: usize = 1 << PGTAB_SHIFT;

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
