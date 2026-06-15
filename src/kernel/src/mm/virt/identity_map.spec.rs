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

//==================================================================================================
// Dependency contracts for not-yet-verified modules
//
// The kernel HAL paging layer (`arch::mem::paging` types/functions) and the kernel
// `page_table_allocator` storage marker are not verified yet. They are registered as external
// types / given trusted external specifications here so that the in-scope identity-map bodies can
// be translated. These are placeholders: when the underlying modules are verified, their real
// `#[verus_spec]` contracts supersede them. The intra-call obligations they would discharge are
// currently `admit()`-ed in the exec bodies, so minimal (state-free) contracts suffice.
//==================================================================================================

use super::page_table_allocator::PageTableBss;

// NOTE: `TableEntry` (trait), `Table<E>`, `TableIndex`, `pd_index`, `pt_index`,
// `Table::from_address`, `Table::read`, and `Table::write` now have real
// `#[verus_spec]`/`#[verus_verify]` contracts in `arch::x86::mem::paging::table`.
// Their former placeholder declarations here (`ExTableEntry`,
// `ExTable`/`ExTableIndex` external type specs, and the `assume_specification`s
// for the index helpers and raw accessors) were removed because the real arch
// specifications now supersede them — per the documented "placeholders are
// removed when the dependency module is verified" methodology.

// --- Page-table structure types ---

// NOTE: `PageTableEntry` and `PageTableEntryFlags` now carry real `#[verus_verify]`
// modeling (View types) in `arch::x86::mem::paging::pte`. Their former placeholder
// external type specifications here (`ExPageTableEntry`, `ExPageTableEntryFlags`)
// were removed because the real arch modeling supersedes them — per the documented
// "placeholders are removed when the dependency module is verified" methodology.

// --- Flag enums ---

// NOTE: `PageDirectoryEntry`, `PageDirectoryEntryFlags`, and the page-table flag
// enums (`PresentFlag`, `ReadWriteFlag`, `UserSupervisorFlag`,
// `PageWriteThroughFlag`, `PageCacheDisableFlag`, `AccessedFlag`, `DirtyFlag`,
// `PageSizeFlag`) now carry real `#[verus_verify]` modeling in
// `arch::x86::mem::paging`. Their former placeholder external type
// specifications here were removed because the real arch modeling supersedes
// them — per the documented "placeholders are removed when the dependency
// module is verified" methodology.

// --- BSS page-table storage marker (kernel `page_table_allocator`) ---

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTableBss(PageTableBss);

// --- Paging index helpers ---

// `pd_index` / `pt_index` are specified in `arch::…::paging::table` (removed here).

// --- TLB maintenance ---

// `invlpg` is specified in `arch::…::paging` (`mod.rs`) — an external-bottom
// hardware trust boundary (`#[verus_verify(external_body)]`, inline-asm TLB flush),
// recorded in `verus-ai-logs/tcb-allowed.md`. The inherited `assume_specification`
// (empty contract) is removed here now that the dependency module provides the
// identical trusted contract.

// --- `Table` raw accessors ---

// `Table::from_address` / `Table::read` / `Table::write` are specified in
// `arch::…::paging::table` (removed here).

// --- Page-directory-entry operations ---

// `PageDirectoryEntryFlags::new`, `PageDirectoryEntry::new`,
// `PageDirectoryEntry::is_present`, and `PageDirectoryEntry::frame_address` now
// carry real `#[verus_spec]` contracts in `arch::x86::mem::paging::pde`, so their
// former placeholder `assume_specification`s here were removed — the real arch
// specifications supersede them.

// --- Page-table-entry operations ---

// `PageTableEntryFlags::new`, `PageTableEntry::new`, and `PageTableEntry::is_present`
// now carry real `#[verus_spec]` contracts in `arch::x86::mem::paging::pte`, so their
// former placeholder `assume_specification`s here were removed — the real arch
// specifications supersede them.

// `<[T]>::as_ptr` no longer needs a placeholder `assume_specification` here: its only
// caller in this module is `ensure_pt`, whose body is `#[verus_verify(external_body)]`
// (TCB-listed) and therefore not translated by Verus. The former declaration was removed
// to shrink the trust surface.

// --- Page-table BSS bump allocator constructor (kernel/`bump_allocator`, not yet verified) ---

pub assume_specification<const N: usize, const A: usize, S: ::bump_allocator::BssStorage>[
    ::bump_allocator::FixedSizeBumpAllocator::<N, A, S>::new
]() -> ::bump_allocator::FixedSizeBumpAllocator<N, A, S>;

} // verus!
