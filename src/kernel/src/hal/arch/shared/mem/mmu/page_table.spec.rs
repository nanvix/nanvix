// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use ::vstd::raw_ptr::PointsTo;

verus! {

/// Mask for the accessed bit in a page-table entry.
#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
pub const PTE_ACCESSED_BIT: PteWord = 1 << 5;

/// Mask for the dirty bit in a page-table entry.
#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
pub const PTE_DIRTY_BIT: PteWord = 1 << 6;

/// Nanvix's authority and stable knowledge for one page-table entry.
/// Fields must not be public, to avoid modification outside the TCB.
#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
pub struct NanvixPteToken {
    ptr: *mut PteWord,
    expected: Option<PteWord>,
}

/// Converts raw-memory permissions into uninitialized page-table-entry tokens.
///
/// This trusted conversion consumes the exact memory permissions, so no competing raw-memory
/// authority remains after the page-table abstraction takes ownership.
#[verifier::external_body]
pub proof fn mint_nanvix_pte_tokens(
    tracked raw_permissions: Map<nat, PointsTo<PteWord>>,
) -> (tracked tokens: Map<nat, NanvixPteToken>)
    requires
        forall|i: nat| raw_permissions.dom().contains(i)
            ==> #[trigger] raw_permissions[i].is_uninit(),
    ensures
        tokens.dom() == raw_permissions.dom(),
        forall|i: nat| raw_permissions.dom().contains(i) ==> {
            let raw_permission = #[trigger] raw_permissions[i];
            let token = #[trigger] tokens[i];

            &&& token.ptr() == raw_permission.ptr()
            &&& token.is_uninit()
        },
{
    unimplemented!()
}

#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
impl NanvixPteToken {
    /// Returns the address of the associated page-table entry.
    pub closed spec fn ptr(&self) -> *mut PteWord {
        self.ptr
    }

    /// Returns the baseline value most recently established by Nanvix.
    pub closed spec fn expected(&self) -> Option<PteWord>
    {
        self.expected
    }

    /// Returns whether Nanvix has established a baseline value.
    pub open spec fn is_init(&self) -> bool {
        self.expected().is_some()
    }

    /// Returns whether Nanvix has not established a baseline value.
    pub open spec fn is_uninit(&self) -> bool {
        self.expected().is_none()
    }

    /// Returns whether `value` may currently be observed at this entry.
    pub open spec fn admits(&self, value: PteWord) -> bool {
        self.is_init() && compatible_pte(self.expected().unwrap(), value)
    }

    /// Returns whether this token is well formed.
    pub closed spec fn wf(&self) -> bool {
        self.is_uninit() || valid_pte(self.expected().unwrap())
    }
}

impl<T> PageTable<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageTableStorage,
{
    spec fn permissions_match_storage(&self) -> bool {
        forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
            let permission = #[trigger] self.permissions[i];

            permission.ptr()@.addr as int
                == self.entries.get_storage().entries_base_address()
                    + i * 4
        }
    }

    pub closed spec fn wf(&self) -> bool {
        &&& self.permissions.dom().len() == ::arch::mem::PAGE_TABLE_LENGTH
        &&& forall|i: nat| self.permissions.dom().contains(i)
            <==> 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH
        &&& forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
            let permission = #[trigger] self.permissions[i];

            permission.wf()
        }
    }

    pub closed spec fn internal_inv(&self) -> bool {
        self.permissions_match_storage()
    }

    pub closed spec fn inv(&self) -> bool {
        self.wf() && self.internal_inv()
    }

    /// Returns the physical base address encoded by a parent page-directory entry.
    pub closed spec fn physical_base(&self) -> int {
        self.entries.get_storage().physical_base_address()
    }

    /// Returns whether the MMU may safely walk this page table.
    pub closed spec fn ready_for_mmu(&self) -> bool {
        &&& self.inv()
        &&& self.permissions.dom().len() == ::arch::mem::PAGE_TABLE_LENGTH
        &&& forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH
            ==> #[trigger] self.permissions[i].is_init()
    }
}

/// Returns whether `value` is an architecturally valid page-table entry.
///
/// All 32-bit values are accepted until feature-dependent reserved-bit rules are modeled.
pub open spec fn valid_pte(_value: PteWord) -> bool {
    true
}

/// Returns the fields that the MMU cannot modify.
pub open spec fn stable_pte_fields(value: PteWord) -> PteWord {
    value & !(PTE_ACCESSED_BIT | PTE_DIRTY_BIT)
}

/// Returns whether `actual` may be observed after Nanvix established `expected`.
pub open spec fn compatible_pte(expected: PteWord, actual: PteWord) -> bool {
    &&& valid_pte(expected)
    &&& valid_pte(actual)
    &&& stable_pte_fields(actual) == stable_pte_fields(expected)
    &&& (expected & PTE_ACCESSED_BIT != 0 ==> actual & PTE_ACCESSED_BIT != 0)
    &&& (expected & PTE_DIRTY_BIT != 0 ==> actual & PTE_DIRTY_BIT != 0)
}

} // verus!

impl<T> PageTable<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageTableStorage,
{
    // Equivalent to direct indexing because it returns the same raw PTE without modifying it.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            self.inv(),
            0 <= index < ::arch::mem::PAGE_TABLE_LENGTH,
            self.permissions[index as nat].is_init(),
        ensures
            self.permissions[index as nat].admits(result),
    )]
    fn env_interaction_read_page_table_entry(&self, index: usize) -> PteWord {
        self.entries[index]
    }

    // Equivalent to direct assignment because it writes the same raw value to the same PTE.
    #[verus_verify(external_body)]
    #[verus_spec(
        requires
            old(self).inv(),
            0 <= index < ::arch::mem::PAGE_TABLE_LENGTH,
            valid_pte(value),
        ensures
            final(self).inv(),
            final(self).nmapped == old(self).nmapped,
            final(self).permissions[index as nat].ptr()
                == old(self).permissions[index as nat].ptr(),
            final(self).permissions[index as nat].is_init(),
            final(self).permissions[index as nat].expected() == Some(value),
            forall|i: nat|
                0 <= i < ::arch::mem::PAGE_TABLE_LENGTH && i != index as nat
                    ==> final(self).permissions[i] == old(self).permissions[i],
    )]
    fn env_interaction_write_page_table_entry(&mut self, index: usize, value: PteWord) {
        self.entries[index] = value;
    }

    // Equivalent to the replaced loop because it writes zero to every raw PTE in the same order.
    #[verus_verify(external_body)]
    #[verus_spec(
        requires
            old(self).inv(),
        ensures
            final(self).inv(),
            final(self).nmapped == old(self).nmapped,
            forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
                let final_permission = #[trigger] final(self).permissions[i];
                let old_permission = old(self).permissions[i];

                &&& final_permission.ptr() == old_permission.ptr()
                &&& final_permission.is_init()
                &&& final_permission.expected() == Some(0)
            },
    )]
    fn env_interaction_clear_page_table(&mut self) {
        for pte in self.entries.iter_mut() {
            *pte = 0;
        }
    }
}
