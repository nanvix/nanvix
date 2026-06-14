verus! {

//==================================================================================================
// Flag projection helpers
//==================================================================================================

// Each paging-control enum is two-valued (`0` = clear, `1 << SHIFT` = set), i.e. isomorphic to
// `bool`. These helpers project the "set" variant to its spec-world `bool` form. Pattern matching
// is confined here so the View and the constructor transitions stay declarative.
pub open spec fn spec_present_set(p: PresentFlag) -> bool {
    p is Present
}

pub open spec fn spec_rw_set(f: ReadWriteFlag) -> bool {
    f is ReadWrite
}

pub open spec fn spec_us_set(f: UserSupervisorFlag) -> bool {
    f is User
}

pub open spec fn spec_pwt_set(f: PageWriteThroughFlag) -> bool {
    f is WriteThrough
}

pub open spec fn spec_pcd_set(f: PageCacheDisableFlag) -> bool {
    f is CacheDisabled
}

pub open spec fn spec_a_set(f: AccessedFlag) -> bool {
    f is Accessed
}

pub open spec fn spec_d_set(f: DirtyFlag) -> bool {
    f is Dirty
}

pub open spec fn spec_ps_set(f: PageSizeFlag) -> bool {
    f is Large
}

//==================================================================================================
// PageDirectoryEntryFlags — abstract value (the eight control bits)
//==================================================================================================

// To a caller a flags bundle is exactly its eight paging-control bits. The bit-packing into the
// raw `PteWord` is hidden (the View is `closed`), realizing the encoding-independence the caller
// analysis demands.
pub struct PdeFlagsView {
    /// Present (P) bit — the entry maps something.
    pub present: bool,
    /// Read/Write (R/W) bit — writes permitted.
    pub writable: bool,
    /// User/Supervisor (U/S) bit — user-mode access permitted.
    pub user: bool,
    /// Page-Write-Through (PWT) bit.
    pub write_through: bool,
    /// Page-Cache-Disable (PCD) bit.
    pub cache_disabled: bool,
    /// Accessed (A) bit.
    pub accessed: bool,
    /// Dirty (D) bit.
    pub dirty: bool,
    /// Page-Size (PS) bit — entry maps a large page.
    pub large_page: bool,
}

impl View for PageDirectoryEntryFlags {
    type V = PdeFlagsView;

    closed spec fn view(&self) -> PdeFlagsView {
        PdeFlagsView {
            present: spec_present_set(self.present),
            writable: spec_rw_set(self.read_write),
            user: spec_us_set(self.user_supervisor),
            write_through: spec_pwt_set(self.page_write_through),
            cache_disabled: spec_pcd_set(self.page_cache_disable),
            accessed: spec_a_set(self.accessed),
            dirty: spec_d_set(self.dirty),
            large_page: spec_ps_set(self.page_size),
        }
    }
}

impl PageDirectoryEntryFlags {
    // A flags bundle has no cross-field constraint: every combination of the eight bits is a legal
    // value, so the invariant is vacuous. Kept explicit for uniformity.
    pub open spec fn inv(&self) -> bool {
        true
    }
}

// Abstract value produced by `PageDirectoryEntryFlags::new`: records each of the eight arguments
// faithfully (caller invariant 1).
pub open spec fn spec_pde_flags_new(
    present: PresentFlag,
    read_write: ReadWriteFlag,
    user_supervisor: UserSupervisorFlag,
    page_write_through: PageWriteThroughFlag,
    page_cache_disable: PageCacheDisableFlag,
    accessed: AccessedFlag,
    dirty: DirtyFlag,
    page_size: PageSizeFlag,
) -> PdeFlagsView {
    PdeFlagsView {
        present: spec_present_set(present),
        writable: spec_rw_set(read_write),
        user: spec_us_set(user_supervisor),
        write_through: spec_pwt_set(page_write_through),
        cache_disabled: spec_pcd_set(page_cache_disable),
        accessed: spec_a_set(accessed),
        dirty: spec_d_set(dirty),
        large_page: spec_ps_set(page_size),
    }
}

//==================================================================================================
// PageDirectoryEntry — abstract value (flags + frame)
//==================================================================================================

// A PDE is the pair `(flags, frame)`. The frame is abstracted as its integer index (the
// `FrameNumber` View); the physical base address it yields is *derived* (`frame * FRAME_SIZE`),
// never stored.
pub struct PdeView {
    /// The eight control bits this entry was built with.
    pub flags: PdeFlagsView,
    /// The frame index this entry points at (== the inner `FrameNumber`'s `@`).
    pub frame: int,
}

impl View for PageDirectoryEntry {
    type V = PdeView;

    closed spec fn view(&self) -> PdeView {
        PdeView { flags: self.flags@, frame: self.frame@ }
    }
}

impl PageDirectoryEntry {
    // The only real constraint is the frame bound, inherited verbatim from the `FrameNumber` type
    // invariant. It is what makes `frame_address` total and overflow-free: the derived physical
    // base `frame * FRAME_SIZE` is well-defined and cannot overflow `usize`. (The flags carry no
    // cross-field constraint — `PageDirectoryEntryFlags::inv` is vacuously `true` — so they add
    // nothing here.)
    pub open spec fn inv(&self) -> bool {
        0 <= self@.frame <= FrameNumber::spec_max()
    }
}

// Abstract value produced by `PageDirectoryEntry::new`: pairs *these exact* flags with *this exact*
// frame (caller invariant 2).
pub open spec fn spec_pde_new(flags: PdeFlagsView, frame: int) -> PdeView {
    PdeView { flags, frame }
}

} // verus!
