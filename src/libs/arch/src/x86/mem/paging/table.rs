// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("table.spec.rs");
#[cfg(verus_keep_ghost)]
include!("table.proof.rs");

use super::{
    PteWord,
    PTE_WORD_SIZE_LOG2,
};

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
#[verus_verify]
pub trait TableEntry: Copy {
    /// Creates from a raw [`PteWord`], returning `None` if the value is invalid.
    fn from_raw(raw: PteWord) -> Option<Self>;
    /// Returns the raw [`PteWord`] representation.
    fn raw(self) -> PteWord;
}

//==================================================================================================
// Table Index
//==================================================================================================

///
/// # Description
///
/// A validated index into a page table, guaranteed to be in `[0, PAGE_TABLE_LENGTH)`.
///
#[verus_verify]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableIndex(usize);

#[verus_verify]
impl TableIndex {
    ///
    /// # Description
    ///
    /// Creates a [`TableIndex`] from a raw `usize`, returning `None` if the value is out of
    /// bounds.
    ///
    #[verus_spec(result =>
        ensures
            match result {
                Some(t) => index < crate::mem::PAGE_TABLE_LENGTH && t@ == index as nat,
                None => index >= crate::mem::PAGE_TABLE_LENGTH,
            },
    )]
    pub const fn new(index: usize) -> Option<Self> {
        if index < crate::mem::PAGE_TABLE_LENGTH {
            Some(Self(index))
        } else {
            None
        }
    }

    ///
    /// # Description
    ///
    /// Returns the underlying index value.
    ///
    #[verus_spec(result =>
        ensures
            result as nat == self@,
            result < crate::mem::PAGE_TABLE_LENGTH,
    )]
    pub const fn into_raw(self) -> usize {
        proof! { use_type_invariant(self); }
        self.0
    }
}

//==================================================================================================
// Virtual Address Index Extraction
//==================================================================================================

/// Extracts the PD index (bits 22-31) from a virtual address as a [`TableIndex`].
#[verus_spec(result =>
    ensures
        result@ == spec_pd_index(vaddr),
        result@ < crate::mem::PAGE_TABLE_LENGTH,
)]
pub const fn pd_index(vaddr: usize) -> TableIndex {
    // The mask guarantees the result is always < PAGE_TABLE_LENGTH.
    let index: usize = (vaddr >> crate::mem::PGTAB_SHIFT) & (crate::mem::PAGE_TABLE_LENGTH - 1);
    proof! { assert(index < crate::mem::PAGE_TABLE_LENGTH) by (bit_vector); }
    TableIndex(index)
}

/// Extracts the PT index (bits 12-21) from a virtual address as a [`TableIndex`].
#[verus_spec(result =>
    ensures
        result@ == spec_pt_index(vaddr),
        result@ < crate::mem::PAGE_TABLE_LENGTH,
)]
pub const fn pt_index(vaddr: usize) -> TableIndex {
    // The mask guarantees the result is always < PAGE_TABLE_LENGTH.
    let index: usize = (vaddr >> crate::mem::PAGE_SHIFT) & (crate::mem::PAGE_TABLE_LENGTH - 1);
    proof! { assert(index < crate::mem::PAGE_TABLE_LENGTH) by (bit_vector); }
    TableIndex(index)
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
/// The table is accessed via a base address that must be valid in the current address space
/// (i.e., a mapped virtual address). It does not own the backing memory — the caller is
/// responsible for allocation and lifetime management.
///
#[verus_verify]
#[derive(Debug)]
pub struct Table<E: TableEntry> {
    /// Base address of the table (must be page-aligned).
    base: usize,
    /// Phantom marker for the entry type.
    _marker: ::core::marker::PhantomData<E>,
}

#[verus_verify]
impl<E: TableEntry> Table<E> {
    ///
    /// # Description
    ///
    /// Creates a table reference from a base address.
    ///
    /// # Safety
    ///
    /// `base` must be a valid, page-aligned address with at least one page of readable/writable
    /// memory.
    ///
    #[verus_spec(result =>
        ensures result@.addr == base as nat,
    )]
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
    /// Returns `None` if the raw value is invalid according to `E::from_raw()`.
    ///
    /// # Safety
    ///
    /// The memory at `base + index * size_of::<PteWord>()` must be valid for a volatile read.
    ///
    // Trust boundary (see `verus-ai-logs/.../tcb-allowed.md` and `verus-unsupported.md`):
    // materializes a raw `*const PteWord` from the integer base address (`usize as *const`) and
    // performs a volatile load — the hardware page-table access. Verus does not support the
    // `usize -> *const T` cast, so the body cannot be verified; this mirrors the
    // `bump_allocator::alloc` / `frame::instance` int-to-pointer materialization boundaries.
    #[verus_verify(external_body)]
    pub unsafe fn read(&self, index: TableIndex) -> Option<E> {
        let offset: usize = index.into_raw() << PTE_WORD_SIZE_LOG2;
        let ptr: *const PteWord = (self.base + offset) as *const PteWord;
        E::from_raw(::core::ptr::read_volatile(ptr))
    }

    ///
    /// # Description
    ///
    /// Writes `entry` at `index`.
    ///
    /// # Safety
    ///
    /// The memory at `base + index * size_of::<PteWord>()` must be valid for a volatile write.
    ///
    // Trust boundary (see `tcb-allowed.md` / `verus-unsupported.md`): materializes a raw
    // `*mut PteWord` from the integer base address and performs a volatile store — the hardware
    // page-table write. The `usize -> *mut T` cast is unsupported by Verus, so the body cannot be
    // verified; same int-to-pointer boundary as `read`.
    #[verus_verify(external_body)]
    pub unsafe fn write(&self, index: TableIndex, entry: E) {
        let offset: usize = index.into_raw() << PTE_WORD_SIZE_LOG2;
        let ptr: *mut PteWord = (self.base + offset) as *mut PteWord;
        ::core::ptr::write_volatile(ptr, entry.raw());
    }
}
