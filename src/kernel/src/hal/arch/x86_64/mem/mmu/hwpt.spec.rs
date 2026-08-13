// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use ::vstd::invariant::{
    InvariantPredicate,
    LocalInvariant,
};
use ::vstd::open_local_invariant;
#[cfg(verus_keep_ghost)]
use ::vstd::raw_ptr::PointsTo;
#[cfg(verus_keep_ghost)]
use ::vstd::shared::Shared;

verus! {

/// Mask for the accessed bit in a hardware paging entry.
#[cfg(verus_keep_ghost_body)]
pub const HW_ACCESSED_BIT: u64 = 1 << 5;

/// Mask for the dirty bit in a hardware paging entry.
#[cfg(verus_keep_ghost_body)]
pub const HW_DIRTY_BIT: u64 = 1 << 6;

/// Mask for the physical address in a 1 GiB page entry.
#[cfg(verus_keep_ghost_body)]
pub const ADDR_MASK_1G: u64 = 0x000F_FFFF_C000_0000;

/// Returns the size of one x86_64 hardware paging entry in bytes.
pub open spec fn hw_entry_size() -> int {
    8
}

/// Level of one page in the x86_64 hardware paging hierarchy.
#[cfg(verus_keep_ghost_body)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HwPagingLevel {
    Pml4,
    Pdpt,
    Pd,
    Pt,
}

/// Nanvix's authority and stable knowledge for one hardware paging entry.
#[cfg(verus_keep_ghost_body)]
pub struct NanvixHwEntryToken {
    pub ptr: *mut u64,
    pub expected: Option<u64>,
}

/// Nanvix's authority and stable knowledge for one hardware paging page.
#[cfg(verus_keep_ghost_body)]
pub struct NanvixHwPageToken {
    pub physical_base: u64,
    pub level: HwPagingLevel,
    pub entries: Map<nat, NanvixHwEntryToken>,
}

/// Proof-only ownership held by the hardware page-table manager.
#[cfg(verus_keep_ghost_body)]
pub struct HwptManagerState {
    pub boot_pages: Map<u64, NanvixHwPageToken>,
    pub available_pages: Map<u64, NanvixHwPageToken>,
}

/// Invariant predicate for the shared hardware page-table manager state.
#[cfg(verus_keep_ghost_body)]
pub struct HwptManagerInvariant;

/// Duplicable proof-only handle to the unique hardware page-table manager state.
#[cfg(verus_keep_ghost_body)]
pub type HwptManagerHandle =
    Shared<LocalInvariant<(), HwptManagerState, HwptManagerInvariant>>;

#[cfg(verus_keep_ghost_body)]
impl InvariantPredicate<(), HwptManagerState> for HwptManagerInvariant {
    open spec fn inv(_constant: (), state: HwptManagerState) -> bool {
        state.inv()
    }
}

/// Converts one initialized raw-memory permission into a hardware-entry token.
///
/// The baseline is the value carried by the consumed permission.
#[verifier::external_body]
pub proof fn mint_nanvix_hw_entry_token(
    tracked raw_permission: PointsTo<u64>,
    level: HwPagingLevel,
) -> (tracked token: NanvixHwEntryToken)
    requires
        raw_permission.is_init(),
        valid_hw_entry(level, raw_permission.value()),
    ensures
        token.ptr() == raw_permission.ptr(),
        token.is_init(),
        token.expected() == raw_permission.value(),
{
    unimplemented!()
}

/// Converts initialized raw-memory permissions into one hardware paging-page token.
///
/// This trusted conversion consumes every entry permission and preserves each current raw value as
/// the corresponding Nanvix baseline.
#[verifier::external_body]
pub proof fn mint_nanvix_hw_page_token(
    tracked raw_permissions: Map<nat, PointsTo<u64>>,
    physical_base: u64,
    level: HwPagingLevel,
) -> (tracked page: NanvixHwPageToken)
    requires
        physical_base % 4096u64 == 0,
        raw_permissions.dom().len() == ENTRIES_PER_TABLE,
        forall|i: nat| raw_permissions.dom().contains(i)
            <==> 0 <= i < ENTRIES_PER_TABLE,
        forall|i: nat| 0 <= i < ENTRIES_PER_TABLE ==> {
            let raw_permission = #[trigger] raw_permissions[i];

            &&& raw_permission.ptr()@.addr as int
                == physical_base as int + i * hw_entry_size()
            &&& raw_permission.is_init()
            &&& valid_hw_entry(level, raw_permission.value())
        },
    ensures
        page.ready_for_mmu(),
        page.physical_base() == physical_base,
        page.level() == level,
        forall|i: nat| 0 <= i < ENTRIES_PER_TABLE ==> {
            let raw_permission = #[trigger] raw_permissions[i];
            let entry = #[trigger] page.entry(i);

            &&& entry.ptr() == raw_permission.ptr()
            &&& entry.expected() == raw_permission.value()
        },
{
    unimplemented!()
}

/// Imports the boot hierarchy and static page-table pool at the one-time initialization boundary.
///
/// The trusted conversion consumes every raw permission. Boot-page tokens remain manager-owned,
/// while pool-page tokens remain available until allocation transfers them to an address space.
#[verifier::external_body]
pub proof fn mint_hwpt_manager(
    tracked boot_permissions: Map<u64, Map<nat, PointsTo<u64>>>,
    boot_levels: Map<u64, HwPagingLevel>,
    tracked pool_permissions: Map<u64, Map<nat, PointsTo<u64>>>,
) -> (tracked manager: HwptManagerHandle)
    requires
        boot_permissions.dom() == boot_levels.dom(),
        boot_permissions.dom().disjoint(pool_permissions.dom()),
        forall|base: u64| boot_permissions.dom().contains(base) ==> {
            let permissions = #[trigger] boot_permissions[base];
            &&& base % 4096u64 == 0
            &&& permissions.dom().len() == ENTRIES_PER_TABLE
            &&& forall|i: nat| permissions.dom().contains(i)
                <==> 0 <= i < ENTRIES_PER_TABLE
            &&& forall|i: nat| 0 <= i < ENTRIES_PER_TABLE ==> {
                let permission = #[trigger] permissions[i];
                &&& permission.ptr()@.addr as int
                    == base as int + i * hw_entry_size()
                &&& permission.is_init()
                &&& valid_hw_entry(boot_levels[base], permission.value())
            }
        },
        forall|base: u64| pool_permissions.dom().contains(base) ==> {
            let permissions = #[trigger] pool_permissions[base];
            &&& base % 4096u64 == 0
            &&& permissions.dom().len() == ENTRIES_PER_TABLE
            &&& forall|i: nat| permissions.dom().contains(i)
                <==> 0 <= i < ENTRIES_PER_TABLE
            &&& forall|i: nat| 0 <= i < ENTRIES_PER_TABLE ==> {
                let permission = #[trigger] permissions[i];
                &&& permission.ptr()@.addr as int
                    == base as int + i * hw_entry_size()
                &&& permission.is_init()
                &&& permission.value() == 0
            }
        },
    ensures
        manager@.constant() == (),
{
    unimplemented!()
}

#[cfg(verus_keep_ghost_body)]
impl NanvixHwEntryToken {
    /// Returns the address of the associated hardware paging entry.
    pub closed spec fn ptr(&self) -> *mut u64 {
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
    pub closed spec fn expected(&self) -> u64
        recommends
            self.is_init(),
    {
        self.expected.unwrap()
    }

    /// Returns whether `value` may currently be observed at this entry.
    pub open spec fn admits(&self, level: HwPagingLevel, value: u64) -> bool {
        self.is_init() && compatible_hw_entry(level, self.expected(), value)
    }

    /// Returns whether this token is well formed for `level`.
    pub open spec fn wf(&self, level: HwPagingLevel) -> bool {
        self.is_uninit() || valid_hw_entry(level, self.expected())
    }
}

#[cfg(verus_keep_ghost_body)]
impl NanvixHwPageToken {
    /// Returns the physical base address of this paging page.
    pub closed spec fn physical_base(&self) -> u64 {
        self.physical_base
    }

    /// Returns the hierarchy level assigned to this paging page.
    pub closed spec fn level(&self) -> HwPagingLevel {
        self.level
    }

    /// Returns the token for entry `index`.
    pub closed spec fn entry(&self, index: nat) -> NanvixHwEntryToken
        recommends
            self.entries.dom().contains(index),
    {
        self.entries[index]
    }

    /// Returns whether the page shape and entry tokens are internally consistent.
    pub open spec fn wf(&self) -> bool {
        &&& self.physical_base % 4096u64 == 0
        &&& self.entries.dom().len() == ENTRIES_PER_TABLE
        &&& forall|i: nat| self.entries.dom().contains(i)
            <==> 0 <= i < ENTRIES_PER_TABLE
        &&& forall|i: nat| 0 <= i < ENTRIES_PER_TABLE ==> {
            let entry = #[trigger] self.entries[i];

            &&& entry.ptr().addr() as int
                == self.physical_base as int + i * hw_entry_size()
            &&& entry.wf(self.level)
        }
    }

    /// Returns whether the MMU may walk this page at its assigned level.
    pub open spec fn ready_for_mmu(&self) -> bool {
        self.wf()
            && forall|i: nat| 0 <= i < ENTRIES_PER_TABLE
                ==> #[trigger] self.entries[i].is_init()
    }

    /// Returns whether every entry has the zero baseline.
    pub open spec fn is_zeroed(&self) -> bool {
        self.ready_for_mmu()
            && forall|i: nat| 0 <= i < ENTRIES_PER_TABLE
                ==> #[trigger] self.entries[i].expected() == 0
    }
}

/// Returns whether every token matches its owner-map key and is ready for MMU access.
pub open spec fn owned_hw_pages_wf(
    pages: Map<u64, NanvixHwPageToken>,
) -> bool {
    forall|base: u64| pages.dom().contains(base) ==> {
        let page = #[trigger] pages[base];
        &&& page.physical_base() == base
        &&& page.ready_for_mmu()
    }
}

/// Returns whether an owner has complete authority for its hardware paging hierarchy.
///
/// The sole permitted external edge is `PDPT[0]`, which points at the manager-owned boot `PD0`
/// shared by every process address space.
pub open spec fn owned_hw_pages_inv(
    pages: Map<u64, NanvixHwPageToken>,
) -> bool {
    &&& owned_hw_pages_wf(pages)
    &&& forall|base: u64| pages.dom().contains(base) ==> {
        let page = #[trigger] pages[base];
        forall|i: nat| 0 <= i < ENTRIES_PER_TABLE
            && hw_entry_nonleaf(page.level(), page.entry(i).expected()) ==> {
                let child_base =
                    hw_entry_target_address(page.entry(i).expected());
                ||| page.level() == HwPagingLevel::Pdpt && i == 0
                ||| {
                    &&& child_base != base
                    &&& pages.dom().contains(child_base)
                    &&& pages[child_base].physical_base() == child_base
                    &&& pages[child_base].level()
                        == next_hw_level(page.level())
                    &&& pages[child_base].ready_for_mmu()
                }
            }
    }
}

/// Returns whether any page in `pages`, other than `base` itself, references `base`.
pub open spec fn owned_hw_page_is_referenced(
    pages: Map<u64, NanvixHwPageToken>,
    base: u64,
) -> bool {
    exists|parent_base: u64, i: nat|
        pages.dom().contains(parent_base)
            && parent_base != base
            && 0 <= i < ENTRIES_PER_TABLE
            && hw_entry_nonleaf(
                pages[parent_base].level(),
                pages[parent_base].entry(i).expected(),
            )
            && hw_entry_target_address(
                pages[parent_base].entry(i).expected(),
            ) == base
}

#[cfg(verus_keep_ghost_body)]
impl HwptManagerState {
    /// Returns whether manager-owned token domains are disjoint and internally valid.
    pub open spec fn inv(&self) -> bool {
        &&& self.boot_pages.dom().disjoint(self.available_pages.dom())
        &&& owned_hw_pages_inv(self.boot_pages)
        &&& forall|base: u64| self.available_pages.dom().contains(base) ==> {
            let page = #[trigger] self.available_pages[base];
            &&& page.physical_base() == base
            &&& page.ready_for_mmu()
        }
    }
}

/// Returns whether `value` is present.
pub open spec fn hw_entry_present(value: u64) -> bool {
    value & PTE_PRESENT != 0
}

/// Returns whether `value` is a large-page entry.
pub open spec fn hw_entry_large(value: u64) -> bool {
    value & PDE_PS != 0
}

/// Returns whether `value` is a structurally valid PML4 entry.
pub open spec fn valid_pml4e(value: u64) -> bool {
    !hw_entry_present(value) || !hw_entry_large(value)
}

/// Returns whether `value` is a structurally valid PDPT entry.
pub open spec fn valid_pdpte(value: u64) -> bool {
    !hw_entry_present(value)
        || (!hw_entry_large(value)
            || value & ADDR_MASK_4K == value & ADDR_MASK_1G)
}

/// Returns whether `value` is a structurally valid PD entry.
pub open spec fn valid_pde(value: u64) -> bool {
    !hw_entry_present(value)
        || (!hw_entry_large(value)
            || value & ADDR_MASK_4K == value & ADDR_MASK_2M)
}

/// Returns whether `value` is a structurally valid PT entry.
pub open spec fn valid_pte(_value: u64) -> bool {
    true
}

/// Returns whether `value` is valid at `level`.
pub open spec fn valid_hw_entry(level: HwPagingLevel, value: u64) -> bool {
    match level {
        HwPagingLevel::Pml4 => valid_pml4e(value),
        HwPagingLevel::Pdpt => valid_pdpte(value),
        HwPagingLevel::Pd => valid_pde(value),
        HwPagingLevel::Pt => valid_pte(value),
    }
}

/// Returns whether a present entry is a leaf mapping at `level`.
pub open spec fn hw_entry_leaf(level: HwPagingLevel, value: u64) -> bool {
    hw_entry_present(value)
        && match level {
            HwPagingLevel::Pml4 => false,
            HwPagingLevel::Pdpt => hw_entry_large(value),
            HwPagingLevel::Pd => hw_entry_large(value),
            HwPagingLevel::Pt => true,
        }
}

/// Returns whether a present entry points to a child paging page.
pub open spec fn hw_entry_nonleaf(level: HwPagingLevel, value: u64) -> bool {
    hw_entry_present(value) && !hw_entry_leaf(level, value)
}

/// Returns the level below `level`.
pub open spec fn next_hw_level(level: HwPagingLevel) -> HwPagingLevel
    recommends
        level != HwPagingLevel::Pt,
{
    match level {
        HwPagingLevel::Pml4 => HwPagingLevel::Pdpt,
        HwPagingLevel::Pdpt => HwPagingLevel::Pd,
        HwPagingLevel::Pd => HwPagingLevel::Pt,
        HwPagingLevel::Pt => HwPagingLevel::Pt,
    }
}

/// Returns the physical child-page address encoded in `value`.
pub open spec fn hw_entry_target_address(value: u64) -> u64 {
    value & ADDR_MASK_4K
}

/// Returns the fields that the MMU may modify at this entry.
pub open spec fn hw_managed_bits(level: HwPagingLevel, value: u64) -> u64 {
    if !hw_entry_present(value) {
        0
    } else if hw_entry_leaf(level, value) {
        HW_ACCESSED_BIT | HW_DIRTY_BIT
    } else {
        HW_ACCESSED_BIT
    }
}

/// Returns whether `actual` may be observed after Nanvix established `expected`.
pub open spec fn compatible_hw_entry(
    level: HwPagingLevel,
    expected: u64,
    actual: u64,
) -> bool {
    let managed = hw_managed_bits(level, expected);

    &&& valid_hw_entry(level, expected)
    &&& valid_hw_entry(level, actual)
    &&& actual & !managed == expected & !managed
    &&& expected & managed & !(actual & managed) == 0
    &&& (!hw_entry_present(expected) ==> actual == expected)
}

/// Returns whether an entry and optional child token describe exactly one valid target case.
pub open spec fn valid_hw_entry_target(
    level: HwPagingLevel,
    value: u64,
    child: Option<&NanvixHwPageToken>,
) -> bool {
    &&& hw_entry_nonleaf(level, value) == child.is_some()
    &&& (hw_entry_nonleaf(level, value)
        ==> (child.unwrap().ready_for_mmu()
            && child.unwrap().level() == next_hw_level(level)
            && child.unwrap().physical_base() == hw_entry_target_address(value)))
}

} // verus!

// Equivalent to the replaced statement because it writes zero to the same entry address.
unsafe fn env_interaction_zero_hardware_page_table_entry(ptr: *mut u64) {
    unsafe {
        ::core::ptr::write_volatile(ptr, 0);
    }
}

// Equivalent to the replaced expression because it performs the same volatile 64-bit read.
unsafe fn env_interaction_read_hardware_page_table_entry(ptr: *const u64) -> u64 {
    unsafe { ::core::ptr::read_volatile(ptr) }
}

// Equivalent to the replaced statement because it performs the same volatile 64-bit write.
unsafe fn env_interaction_write_hardware_page_table_entry(ptr: *mut u64, value: u64) {
    unsafe {
        ::core::ptr::write_volatile(ptr, value);
    }
}

// Equivalent to the replaced instruction because it invalidates the same virtual page.
unsafe fn env_interaction_invalidate_tlb_page(vaddr: usize) {
    unsafe {
        ::core::arch::asm!(
            "invlpg [{}]",
            in(reg) vaddr,
            options(nostack, preserves_flags)
        );
    }
}

// Equivalent to the replaced instruction because it returns the same current CR3 value.
unsafe fn env_interaction_read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        ::core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, nomem));
    }
    cr3
}
