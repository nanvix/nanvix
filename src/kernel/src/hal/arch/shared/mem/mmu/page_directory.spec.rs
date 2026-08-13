// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use ::vstd::raw_ptr::PointsTo;

verus! {

#[verifier::external_type_specification]
#[verifier::external_body]
#[allow(dead_code)]
pub struct ExPageTableAddress(PageTableAddress);

#[verifier::external_type_specification]
#[verifier::external_body]
#[allow(dead_code)]
pub struct ExPageDirectoryEntry(PageDirectoryEntry);

/// Mask for the accessed bit in a page-directory entry.
#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
pub const ACCESSED_BIT: PteWord = 1 << 5;

/// Mask for the present bit in a page-directory entry.
#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
pub const PRESENT_BIT: PteWord = 1;

/// Mask for the page-size bit in a page-directory entry.
#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
pub const PAGE_SIZE_BIT: PteWord = 1 << 7;

/// Mask for the physical page-table address in a standard page-directory entry.
#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
pub const PAGE_TABLE_ADDRESS_MASK: PteWord = 0xffff_f000;

/// Nanvix's authority and stable knowledge for one page-directory entry.
#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
pub struct NanvixPdeToken {
    pub ptr: *mut PteWord,
    pub expected: Option<PteWord>,
}

/// Converts raw-memory permissions into uninitialized page-directory-entry tokens.
///
/// This trusted conversion consumes the exact memory permissions, so no competing raw-memory
/// authority remains after the page-directory abstraction takes ownership.
#[verifier::external_body]
pub proof fn mint_nanvix_pde_tokens(
    tracked raw_permissions: Map<nat, PointsTo<PteWord>>,
) -> (tracked tokens: Map<nat, NanvixPdeToken>)
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
impl NanvixPdeToken {
    /// Returns the address of the associated page-directory entry.
    pub closed spec fn ptr(&self) -> *mut PteWord {
        self.ptr
    }

    /// Returns whether Nanvix has established a baseline value.
    pub open spec fn is_init(&self) -> bool {
        self.expected.is_some()
    }

    /// Returns whether Nanvix has not established a baseline value.
    pub open spec fn is_uninit(&self) -> bool {
        self.expected.is_none()
    }

    /// Returns the baseline value most recently established by Nanvix.
    pub closed spec fn expected(&self) -> PteWord
        recommends
            self.is_init(),
    {
        self.expected.unwrap()
    }

    /// Returns whether `value` may currently be observed at this entry.
    pub open spec fn admits(&self, value: PteWord) -> bool {
        self.is_init() && compatible_pde(self.expected(), value)
    }

    /// Returns whether this token is well formed.
    pub closed spec fn wf(&self) -> bool {
        self.is_uninit() || valid_standard_pde(self.expected())
    }
}

impl<T> PageDirectory<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageDirectoryStorage,
{
    spec fn permissions_match_storage(&self) -> bool {
        forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
            let permission = #[trigger] self.permissions[i];

            permission.ptr().addr() as int
                == self.entries.get_storage().base_address()
                    + i * 4
        }
    }

    pub closed spec fn wf(&self) -> bool {
        &&& self.permissions.dom().len() == ::arch::mem::PAGE_TABLE_LENGTH
        &&& forall|i: nat| self.permissions.dom().contains(i)
            <==> 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH
        &&& forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
            self.permissions[i].wf()
        }
    }

    pub closed spec fn internal_inv(&self) -> bool {
        self.permissions_match_storage()
    }

    pub closed spec fn inv(&self) -> bool {
        self.wf() && self.internal_inv()
    }

    /// Returns whether every page-directory entry has an initialized baseline.
    pub closed spec fn ready_for_mmu(&self) -> bool {
        &&& self.inv()
        &&& forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH
            ==> #[trigger] self.permissions[i].is_init()
    }
}

pub open spec fn valid_standard_pde(value: PteWord) -> bool {
    value & PAGE_SIZE_BIT == 0
}

/// Returns the fields that the MMU cannot modify.
pub open spec fn stable_pde_fields(value: PteWord) -> PteWord {
    value & !ACCESSED_BIT
}

/// Returns whether `actual` may be observed after Nanvix established `expected`.
pub open spec fn compatible_pde(expected: PteWord, actual: PteWord) -> bool {
    &&& valid_standard_pde(expected)
    &&& valid_standard_pde(actual)
    &&& stable_pde_fields(actual) == stable_pde_fields(expected)
    &&& (expected & ACCESSED_BIT != 0 ==> actual & ACCESSED_BIT != 0)
}

/// Returns whether the MMU follows this page-directory entry.
pub open spec fn present_pde(value: PteWord) -> bool {
    value & PRESENT_BIT != 0
}

/// Returns the physical page-table address encoded in a standard page-directory entry.
pub open spec fn pde_page_table_address(value: PteWord) -> int {
    (value & PAGE_TABLE_ADDRESS_MASK) as int
}

/// Returns whether the PDE and page-table argument describe exactly one valid target case.
pub open spec fn valid_pde_target(
    value: PteWord,
    page_table: Option<&PageTable<PageTableStorage>>,
) -> bool {
    &&& present_pde(value) == page_table.is_some()
    &&& (present_pde(value)
        ==> (page_table.is_some()
            && page_table.unwrap().ready_for_mmu()
            && page_table.unwrap().physical_base() == pde_page_table_address(value)))
}

} // verus!

impl<T> PageDirectory<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageDirectoryStorage,
{
    // Equivalent to the replaced loop because it writes zero to every raw PDE in the same order.
    #[verus_verify(external_body)]
    #[verus_spec(
        requires
            old(self).inv(),
        ensures
            final(self).inv(),
            forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
                &&& final(self).permissions[i].ptr() == old(self).permissions[i].ptr()
                &&& final(self).permissions[i].is_init()
                &&& final(self).permissions[i].expected() == 0
            },
    )]
    fn env_interaction_clear_page_directory(&mut self) {
        for pde in self.entries.iter_mut() {
            *pde = 0;
        }
    }

    // Equivalent to direct indexing because it returns the same raw PDE without modifying it.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            self.inv(),
            0 <= index < ::arch::mem::PAGE_TABLE_LENGTH,
            self.permissions[index as nat].is_init(),
        ensures
            self.permissions[index as nat].admits(result),
    )]
    fn env_interaction_read_page_directory_entry(&self, index: usize) -> PteWord {
        self.entries[index]
    }

    // Equivalent to direct assignment because it writes the same raw value to the same PDE.
    #[verus_verify(external_body)]
    #[verus_spec(
        with
            Ghost(page_table):
                Ghost<Option<&PageTable<PageTableStorage>>>,
        requires
            old(self).inv(),
            0 <= index < ::arch::mem::PAGE_TABLE_LENGTH,
            valid_standard_pde(value),
            valid_pde_target(value, page_table),
        ensures
            final(self).inv(),
            final(self).permissions[index as nat].ptr()
                == old(self).permissions[index as nat].ptr(),
            final(self).permissions[index as nat].is_init(),
            final(self).permissions[index as nat].expected() == value,
            forall|i: nat|
                0 <= i < ::arch::mem::PAGE_TABLE_LENGTH && i != index as nat
                    ==> final(self).permissions[i] == old(self).permissions[i],
    )]
    fn env_interaction_write_page_directory_entry(
        &mut self,
        index: usize,
        value: PteWord,
    ) {
        self.entries[index] = value;
    }
}
