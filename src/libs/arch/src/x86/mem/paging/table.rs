// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::PteWord;

//==================================================================================================
// Table Entry Trait
//==================================================================================================

///
/// # Description
///
/// Trait bound for entry types that can be stored in a page table.
///
/// The raw representation uses [`PteWord`] — `u32` on x86.
///
pub trait TableEntry: Copy {
    /// Creates from a raw [`PteWord`], returning `None` if the value is invalid.
    fn from_raw(raw: PteWord) -> Option<Self>;
    /// Returns the raw [`PteWord`] representation.
    fn raw(self) -> PteWord;
}

//==================================================================================================
// Virtual Address Index Extraction
//==================================================================================================

/// Extracts the PD index (bits 22-31) from a virtual address.
pub const fn pd_index(vaddr: usize) -> usize {
    (vaddr >> crate::mem::PGTAB_SHIFT) & (crate::mem::PGTAB_SIZE / crate::mem::PAGE_SIZE - 1)
}

/// Extracts the PT index (bits 12-21) from a virtual address.
pub const fn pt_index(vaddr: usize) -> usize {
    (vaddr >> crate::mem::PAGE_SHIFT) & (crate::mem::PGTAB_SIZE / crate::mem::PAGE_SIZE - 1)
}

//==================================================================================================
// Table
//==================================================================================================

///
/// # Description
///
/// A page-table page containing [`PAGE_TABLE_LENGTH`](crate::mem::PAGE_TABLE_LENGTH) entries.
///
/// This type is parameterized over the entry type to enforce that each table level uses only
/// its own entry kind (e.g., PD tables only [`PageDirectoryEntry`], PT tables only
/// [`PageTableEntry`]).
///
/// The table is accessed via a physical base address (identity-mapped in kernel space). It does
/// not own the backing memory — the caller is responsible for allocation and lifetime management.
///
#[derive(Debug)]
pub struct Table<E: TableEntry> {
    /// Base address of the table (must be page-aligned, identity-mapped).
    base: usize,
    /// Phantom marker for the entry type.
    _marker: ::core::marker::PhantomData<E>,
}

impl<E: TableEntry> Table<E> {
    ///
    /// # Description
    ///
    /// Creates a table reference from a base address.
    ///
    /// # Safety
    ///
    /// `base` must be a valid, page-aligned, identity-mapped address with at least one page
    /// of readable/writable memory.
    ///
    pub const unsafe fn from_address(base: usize) -> Self {
        Self {
            base,
            _marker: ::core::marker::PhantomData,
        }
    }

    ///
    /// # Description
    ///
    /// Reads the entry at `index`.
    ///
    /// Returns `Err(())` if `index >= PAGE_TABLE_LENGTH`, or `Ok(None)` if the raw value is
    /// invalid according to `E::from_raw()`.
    ///
    /// # Safety
    ///
    /// The memory at `base + index * size_of::<PteWord>()` must be valid for a volatile read.
    ///
    pub unsafe fn read(&self, index: usize) -> Result<Option<E>, ()> {
        if index >= crate::mem::PAGE_TABLE_LENGTH {
            return Err(());
        }
        let ptr: *const PteWord =
            (self.base + index * ::core::mem::size_of::<PteWord>()) as *const PteWord;
        Ok(E::from_raw(::core::ptr::read_volatile(ptr)))
    }

    ///
    /// # Description
    ///
    /// Writes `entry` at `index`.
    ///
    /// Returns `Err(())` if `index >= PAGE_TABLE_LENGTH`.
    ///
    /// # Safety
    ///
    /// The memory at `base + index * size_of::<PteWord>()` must be valid for a volatile write.
    ///
    pub unsafe fn write(&self, index: usize, entry: E) -> Result<(), ()> {
        if index >= crate::mem::PAGE_TABLE_LENGTH {
            return Err(());
        }
        let ptr: *mut PteWord =
            (self.base + index * ::core::mem::size_of::<PteWord>()) as *mut PteWord;
        ::core::ptr::write_volatile(ptr, entry.raw());
        Ok(())
    }
}
