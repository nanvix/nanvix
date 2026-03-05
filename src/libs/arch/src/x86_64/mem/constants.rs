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
/// Log2 PAGE_SIZE.
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
/// Number of entries in a page table (each level).
/// In x86_64, each table has 512 entries (9 bits per level).
///
pub const PAGE_TABLE_ENTRIES: usize = 512;

///
/// # Description
///
/// Log2 of the coverage of a single page table (PT).
/// A page table maps 512 * 4 KB = 2 MB.
///
pub const PGTAB_SHIFT: usize = 21;

///
/// # Description
///
/// Number of bytes covered by a single page table.
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
/// Log2 of the coverage of a page directory (PD).
/// A page directory maps 512 * 2 MB = 1 GB.
///
pub const PGDIR_SHIFT: usize = 30;

///
/// # Description
///
/// Number of bytes covered by a single page directory.
///
pub const PGDIR_SIZE: usize = 1 << PGDIR_SHIFT;

///
/// # Description
///
/// Log2 of the coverage of a PDPT.
/// A PDPT maps 512 * 1 GB = 512 GB.
///
pub const PDPT_SHIFT: usize = 39;

///
/// # Description
///
/// Number of bytes covered by a PDPT.
///
pub const PDPT_SIZE: usize = 1 << PDPT_SHIFT;

///
/// # Description
///
/// Number of bits in a virtual address used for translation (canonical addressing).
///
pub const VIRTUAL_ADDRESS_BITS: usize = 48;

///
/// # Description
///
/// Maximum physical address bits supported by x86_64.
///
pub const PHYSICAL_ADDRESS_BITS: usize = 52;

///
/// # Description
///
/// Maximum addressable physical memory.
///
pub const MAX_PHYSICAL_ADDRESS: u64 = (1u64 << PHYSICAL_ADDRESS_BITS) - 1;

///
/// # Description
///
/// Mask for extracting the physical address from a page table entry.
/// Bits 51:12 hold the physical frame address.
///
pub const PAGE_ENTRY_ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

///
/// # Description
///
/// Alias for `PAGE_SHIFT`.
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
/// Alignment for a page table's virtual address coverage (512 × 4 KB = 2 MB).
///
pub const PGTAB_ALIGNMENT: Alignment = Alignment::Align2097152;
