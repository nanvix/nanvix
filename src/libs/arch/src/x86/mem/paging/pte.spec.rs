verus! {

//==================================================================================================
// Flag projection helpers
//==================================================================================================

// The seven flag projections shared with the sibling `pde` module (`spec_present_set`,
// `spec_rw_set`, `spec_us_set`, `spec_pwt_set`, `spec_pcd_set`, `spec_a_set`, `spec_d_set`) are
// reused from there to avoid duplicate definitions colliding through the `paging` glob re-export.
use crate::x86::mem::paging::{
    spec_present_set,
    spec_rw_set,
    spec_us_set,
    spec_pwt_set,
    spec_pcd_set,
    spec_a_set,
    spec_d_set,
};

// The copy-on-write projection is PTE-specific (the PDE sibling has a page-size bit instead), so it
// is defined here. The enum is two-valued (`0` = clear, `1 << SHIFT` = set), isomorphic to `bool`.
pub open spec fn spec_cow_set(f: CopyOnWriteFlag) -> bool {
    f is CopyOnWrite
}

//==================================================================================================
// PageTableEntryFlags — abstract value (the eight control bits)
//==================================================================================================

// To a caller a flags bundle is exactly its eight paging-control bits. The bit-packing into the
// raw `PteWord` is hidden (the View is `closed`), realizing the encoding-independence the caller
// analysis demands. The single structural difference from the `pde` sibling is the OS-defined
// `cow` (copy-on-write, AVL) bit in place of the PDE's hardware `large_page` (PS) bit.
pub struct PteFlagsView {
    /// Present (P) bit — the entry maps a page (`is_present`).
    pub present: bool,
    /// Read/Write (R/W) bit — writes permitted (`is_writable`).
    pub writable: bool,
    /// User/Supervisor (U/S) bit — user-mode access permitted (`is_user`).
    pub user: bool,
    /// Page-Write-Through (PWT) bit.
    pub write_through: bool,
    /// Page-Cache-Disable (PCD) bit.
    pub cache_disabled: bool,
    /// Accessed (A) bit.
    pub accessed: bool,
    /// Dirty (D) bit.
    pub dirty: bool,
    /// Copy-on-write (OS-defined AVL) bit — set by `set_cow`, read by `is_cow`.
    pub cow: bool,
}

impl View for PageTableEntryFlags {
    type V = PteFlagsView;

    closed spec fn view(&self) -> PteFlagsView {
        PteFlagsView {
            present: spec_present_set(self.present),
            writable: spec_rw_set(self.read_write),
            user: spec_us_set(self.user_supervisor),
            write_through: spec_pwt_set(self.page_write_through),
            cache_disabled: spec_pcd_set(self.page_cache_disable),
            accessed: spec_a_set(self.accessed),
            dirty: spec_d_set(self.dirty),
            cow: spec_cow_set(self.cow),
        }
    }
}

impl PageTableEntryFlags {
    // A flags bundle has no cross-field constraint: every combination of the eight bits is a legal
    // value, so the invariant is vacuous. Kept explicit for uniformity.
    pub open spec fn inv(&self) -> bool {
        true
    }
}

// Abstract value produced by `PageTableEntryFlags::new`: records each of the seven argument bits
// faithfully and defaults the OS-defined copy-on-write bit to `false` (`NotCopyOnWrite`), since it
// is not a parameter (caller invariant: callers rely on `cow` defaulting to `NotCopyOnWrite`).
pub open spec fn spec_pte_flags_new(
    present: PresentFlag,
    read_write: ReadWriteFlag,
    user_supervisor: UserSupervisorFlag,
    page_write_through: PageWriteThroughFlag,
    page_cache_disable: PageCacheDisableFlag,
    accessed: AccessedFlag,
    dirty: DirtyFlag,
) -> PteFlagsView {
    PteFlagsView {
        present: spec_present_set(present),
        writable: spec_rw_set(read_write),
        user: spec_us_set(user_supervisor),
        write_through: spec_pwt_set(page_write_through),
        cache_disabled: spec_pcd_set(page_cache_disable),
        accessed: spec_a_set(accessed),
        dirty: spec_d_set(dirty),
        cow: false,
    }
}

//==================================================================================================
// PageTableEntry — abstract value (flags + frame)
//==================================================================================================

// A PTE is the pair `(flags, frame)`. The frame is abstracted as its integer index (the
// `FrameNumber` View); the physical base address it yields is *derived* (`frame * FRAME_SIZE`),
// never stored.
pub struct PteView {
    /// The eight control bits this entry was built with.
    pub flags: PteFlagsView,
    /// The frame index this entry points at (== the inner `FrameNumber`'s `@`).
    pub frame: int,
}

impl View for PageTableEntry {
    type V = PteView;

    closed spec fn view(&self) -> PteView {
        PteView { flags: self.flags@, frame: self.frame@ }
    }
}

impl PageTableEntry {
    // The only real constraint is the frame bound, inherited verbatim from the `FrameNumber` type
    // invariant. It is what makes the (out-of-scope) `frame_address` total and overflow-free: the
    // derived physical base `frame * FRAME_SIZE` is well-defined and cannot overflow `usize`. (The
    // flags carry no cross-field constraint — `PageTableEntryFlags::inv` is vacuously `true` — so
    // they add nothing here.)
    pub open spec fn inv(&self) -> bool {
        0 <= self@.frame <= FrameNumber::spec_max()
    }
}

// Abstract value produced by `PageTableEntry::new`: pairs *these exact* flags with *this exact*
// frame (caller invariant: constructor fidelity).
pub open spec fn spec_pte_new(flags: PteFlagsView, frame: int) -> PteView {
    PteView { flags, frame }
}

} // verus!
