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

use ::arch::mem::paging::TableEntry;
use super::page_table_allocator::PageTableBss;

// --- Page-table structure types ---

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(E)]
pub struct ExTable<E: TableEntry>(Table<E>);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExTableIndex(TableIndex);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageDirectoryEntry(PageDirectoryEntry);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTableEntry(PageTableEntry);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageDirectoryEntryFlags(PageDirectoryEntryFlags);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTableEntryFlags(PageTableEntryFlags);

// --- Flag enums ---

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPresentFlag(PresentFlag);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExReadWriteFlag(ReadWriteFlag);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExUserSupervisorFlag(UserSupervisorFlag);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageWriteThroughFlag(PageWriteThroughFlag);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageCacheDisableFlag(PageCacheDisableFlag);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExAccessedFlag(AccessedFlag);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExDirtyFlag(DirtyFlag);

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageSizeFlag(PageSizeFlag);

// --- BSS page-table storage marker (kernel `page_table_allocator`) ---

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPageTableBss(PageTableBss);

// --- Paging index helpers ---

pub assume_specification[ ::arch::mem::paging::pd_index ](vaddr: usize) -> TableIndex;

pub assume_specification[ ::arch::mem::paging::pt_index ](vaddr: usize) -> TableIndex;

// --- TLB maintenance ---

pub assume_specification[ ::arch::mem::paging::invlpg ](vaddr: usize);

// --- `Table` raw accessors (trusted HAL boundary) ---

pub assume_specification<E: TableEntry>[ Table::<E>::from_address ](base: usize) -> Table<E>;

pub assume_specification<E: TableEntry>[ Table::<E>::read ](
    table: &Table<E>,
    index: TableIndex,
) -> Option<E>;

pub assume_specification<E: TableEntry>[ Table::<E>::write ](
    table: &Table<E>,
    index: TableIndex,
    entry: E,
);

// --- Page-directory-entry operations ---

pub assume_specification[ PageDirectoryEntryFlags::new ](
    present: PresentFlag,
    read_write: ReadWriteFlag,
    user_supervisor: UserSupervisorFlag,
    page_write_through: PageWriteThroughFlag,
    page_cache_disable: PageCacheDisableFlag,
    accessed: AccessedFlag,
    dirty: DirtyFlag,
    page_size: PageSizeFlag,
) -> PageDirectoryEntryFlags;

pub assume_specification[ PageDirectoryEntry::new ](
    flags: PageDirectoryEntryFlags,
    frame: FrameNumber,
) -> PageDirectoryEntry;

pub assume_specification[ PageDirectoryEntry::is_present ](pde: &PageDirectoryEntry) -> bool;

pub assume_specification[ PageDirectoryEntry::frame_address ](pde: &PageDirectoryEntry) -> usize;

// --- Page-table-entry operations ---

pub assume_specification[ PageTableEntryFlags::new ](
    present: PresentFlag,
    read_write: ReadWriteFlag,
    user_supervisor: UserSupervisorFlag,
    page_write_through: PageWriteThroughFlag,
    page_cache_disable: PageCacheDisableFlag,
    accessed: AccessedFlag,
    dirty: DirtyFlag,
) -> PageTableEntryFlags;

pub assume_specification[ PageTableEntry::new ](
    flags: PageTableEntryFlags,
    frame: FrameNumber,
) -> PageTableEntry;

pub assume_specification[ PageTableEntry::is_present ](pte: &PageTableEntry) -> bool;

// --- Slice base pointer (std, not covered by vstd) ---

pub assume_specification<T>[ <[T]>::as_ptr ](s: &[T]) -> *const T;

} // verus!
