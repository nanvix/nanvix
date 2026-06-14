use crate::hal::mem::spec_page_size;

verus! {

//==================================================================================================
// Abstract state of the kernel identity map
//==================================================================================================

/// Abstract state of the kernel identity map (see `view_design.md`).
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
/// Uninterpreted accessor: the identity-map state lives in module-level singletons
/// (`KERNEL_PD_PADDR` / `KERNEL_CR3` and the BSS page-table pool) whose value is not directly
/// spec-readable. The cross-call transition (`v -> v'`) is realized in the proving phase by a
/// ghost token over those singletons; during the specification phase it is read like `self@`.
/// This mirrors `mm::phys::phys_view()` -- the same singleton-global boundary, not a verification
/// escape.
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
    /// Implementation-consistency invariant (e.g. the abstract `mapped` set agrees with the
    /// present PDE/PTE bits and the page-table-pool bounds). Placeholder during specification;
    /// the proving phase fills it in once the concrete `view()` body is realized.
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

} // verus!
