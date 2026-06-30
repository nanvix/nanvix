use crate::hal::mem::spec_page_size;

verus! {

//==================================================================================================
// Abstract state of the kernel identity map
//==================================================================================================

/// Abstract state of the kernel identity map.
///
/// This is a ghost model of *global* state: the in-scope functions are free functions backed by
/// the `KERNEL_PD_PADDR` / `KERNEL_CR3` atomics plus a BSS page-table pool, so there is no owning
/// exec struct. Addresses use `int` (the `View` of `PhysicalAddress` / `PageAligned`), consistent
/// with the sibling `mm::phys` abstraction (`FrameAllocView` keys frames by `int`).
pub ghost struct IdentityMapView {
    /// Whether the lazy identity mapper has been initialized (i.e. `init` has published the kernel
    /// page directory). Before initialization the boot page tables are still active and every
    /// `identity_map_page` is a successful no-op, so this flag selects which transition applies.
    pub initialized: bool,

    /// The set of identity-mapped pages, each identified by its page-aligned physical base
    /// address. Membership means: the page is **present, writable, supervisor-only**, and
    /// reachable at its own physical address in the kernel address space. Permissions are uniform
    /// across all mapped pages, so they are encoded by membership rather than stored per page.
    pub mapped: Set<int>,
}

/// Current abstract state of the global kernel identity map.
///
/// Uninterpreted accessor for module-level identity-map state.
pub uninterp spec fn identity_map_view() -> IdentityMapView;

//==================================================================================================
// Page-address vocabulary
//==================================================================================================

/// An address is page-aligned when it is a multiple of the page size.
pub open spec fn spec_is_page_aligned(addr: int) -> bool {
    addr % spec_page_size() == 0
}

/// The page-aligned base address of the page containing `addr`.
pub open spec fn spec_page_base(addr: int) -> int {
    addr - (addr % spec_page_size())
}

//==================================================================================================
// Invariants and queries
//==================================================================================================

impl IdentityMapView {
    /// Implementation-consistency invariant.
    pub open spec fn internal_inv(self) -> bool {
        true
    }

    /// Well-formedness invariant callers rely on.
    pub open spec fn inv(self) -> bool {
        &&& self.internal_inv()
        // Every recorded page is identified by a page-aligned base address.
        &&& (forall|p: int| #[trigger] self.mapped.contains(p) ==> spec_is_page_aligned(p))
        // Before initialization no lazy mapping has been installed; coverage is provided entirely
        // by the boot page tables.
        &&& (!self.initialized ==> self.mapped =~= Set::empty())
    }

    /// The page based at `page` is reachable at its own physical address right now. Before init the
    /// boot tables make every page reachable; after init reachability is exactly membership in
    /// `mapped`. This is the headline resource callers obtain from `identity_map_page` on `Ok`.
    pub open spec fn accessible(self, page: int) -> bool {
        !self.initialized || self.mapped.contains(page)
    }

    /// Unconditional install of one page (idempotent). Models the leaf step `ensure_pte` realizes:
    /// it always adds the page to the map.
    pub open spec fn spec_install_page(self, page: int) -> IdentityMapView {
        IdentityMapView { mapped: self.mapped.insert(page), ..self }
    }

    /// Full effect of `identity_map_page`: install the page when the mapper is initialized,
    /// otherwise a no-op (boot tables already cover it). Either way the page ends up `accessible`.
    pub open spec fn spec_map_page(self, page: int) -> IdentityMapView {
        if self.initialized {
            self.spec_install_page(page)
        } else {
            self
        }
    }
}

//==================================================================================================
// Dependency contracts
//==================================================================================================

use super::page_table_allocator::PageTableBss;

// The paging types and helpers are specified in `arch::x86::mem::paging`.

// --- BSS page-table storage marker (kernel `page_table_allocator`) ---

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTableBss(PageTableBss);

// --- Paging index helpers ---

// `pd_index` / `pt_index` are specified in `arch::…::paging::table` (removed here).

// --- TLB maintenance ---

// `invlpg` is specified in `arch::x86::mem::paging`.

// --- `Table` raw accessors ---

// `Table::from_address` / `Table::read` / `Table::write` are specified in
// `arch::…::paging::table` (removed here).

// --- Page-directory-entry operations ---

// Page-directory-entry operations are specified in `arch::x86::mem::paging::pde`.

// --- Page-table-entry operations ---

// Page-table-entry operations are specified in `arch::x86::mem::paging::pte`.

} // verus!
