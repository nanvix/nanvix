verus! {

//==================================================================================================
// TableIndex — a validated page-table slot number
//==================================================================================================

// A `TableIndex` carries exactly one piece of caller-visible information: the validated index
// value. Its abstract value is a single `nat` (see `view_design.md`).
impl View for TableIndex {
    type V = nat;

    // `closed`: callers reference `self@` and the in-range invariant; the mapping to the inner
    // `usize` field is hidden. The abstract value is "the validated slot number".
    closed spec fn view(&self) -> nat {
        self.0 as nat
    }
}

impl TableIndex {
    // The validated-range guarantee every caller depends on: a `TableIndex` is always a legal
    // slot. Enforced as a type invariant so the bound holds unconditionally for any value callers
    // hold — this is exactly what keeps `read`/`write` (and `gva.rs`'s `checked_mul`) within one
    // page, and is established once at each construction site (`new`, `pd_index`, `pt_index`).
    #[verifier::type_invariant]
    pub open spec fn inv(&self) -> bool {
        self@ < crate::mem::PAGE_TABLE_LENGTH
    }
}

//==================================================================================================
// Virtual-address index extraction (pure helpers)
//==================================================================================================

// Low `log2(PAGE_TABLE_LENGTH)` bits of `vaddr >> shift`. Mirrors the exec masking
// `(vaddr >> shift) & (PAGE_TABLE_LENGTH - 1)` exactly; the mask makes the result `< LEN`.
pub open spec fn spec_table_index(vaddr: usize, shift: usize) -> nat {
    ((vaddr >> shift) & ((crate::mem::PAGE_TABLE_LENGTH - 1) as usize)) as nat
}

// PD index = bits [PGTAB_SHIFT, PGTAB_SHIFT + log2(LEN)) of `vaddr`.
pub open spec fn spec_pd_index(vaddr: usize) -> nat {
    spec_table_index(vaddr, crate::mem::PGTAB_SHIFT)
}

// PT index = bits [PAGE_SHIFT, PAGE_SHIFT + log2(LEN)) of `vaddr`.
pub open spec fn spec_pt_index(vaddr: usize) -> nat {
    spec_table_index(vaddr, crate::mem::PAGE_SHIFT)
}

//==================================================================================================
// TableEntry codec (the per-entry round-trip law)
//==================================================================================================

// Abstract serialization / decoding of a `TableEntry`. These mirror the exec `raw` / `from_raw`
// trait methods. They are deliberately *unbounded* over `E` (no `TableEntry` bound) to avoid a
// definitional cycle (`TableEntry`'s method specs reference these, so a bound here would make the
// trait depend on a function that depends on the trait).
//
// `spec_entry_from_raw` returns `None` for a word that is not a valid encoding of `E` — exactly
// the `read` failure path callers map to `InvalidArgument` (see `caller_analysis.md`).
pub uninterp spec fn spec_entry_raw<E>(e: E) -> PteWord;

pub uninterp spec fn spec_entry_from_raw<E>(w: PteWord) -> Option<E>;

//==================================================================================================
// Global ghost model of page-table memory
//==================================================================================================

// Page-table memory is volatile, caller-owned storage the `Table<E>` handle does not contain
// (the struct holds only `base`). It is modeled here as a *global, parameter-free* ghost
// function — the same device used for the physical-frame subsystem (`mm::phys::phys_view()`):
// the abstract state is named globally rather than threaded through a permission parameter, so
// `read`/`write` keep their exec signatures and do not cascade ghost arguments into out-of-scope
// callers (`identity_map::ensure_pt`/`ensure_pte`/`identity_map_page`).
//
// `spec_table_word(addr, index)` is the raw word currently stored at slot `index` of the
// page-table page based at `addr`. Because it is a pure function, a call to `write` only updates
// knowledge about the *one* slot named in its `ensures`; every other slot/page is automatically
// preserved across the call (the caller-facing frame condition). The cross-call write transition
// (and its consistency when the same slot is written twice) is realized in the proving phase by a
// ghost token over the page-table pages — exactly the `phys_view()` "transition realized in the
// proving phase" placeholder.
pub uninterp spec fn spec_table_word(addr: nat, index: nat) -> PteWord;

// The decoded entry currently stored at slot `index` of the page based at `addr`: the value a
// `read` returns. This is the `index -> Option<E>` entry map from `view_design.md`, expressed
// pointwise over the global word store.
pub open spec fn spec_table_read<E>(addr: nat, index: nat) -> Option<E> {
    spec_entry_from_raw::<E>(spec_table_word(addr, index))
}

//==================================================================================================
// Table<E> — a typed, non-owning handle over one page-table page
//==================================================================================================

// To callers a `Table<E>` is a typed handle over the page at a physical/identity-mapped base
// address (see `view_design.md`): its caller-meaningful identity is *which page* read/write act
// on (`addr`), and its observable contents are the per-slot decoded entry map (`entries`). The
// contents live in volatile, caller-owned memory the struct does not store, so the view reads
// them from the global `spec_table_read` ghost rather than from struct fields.
pub struct TableView<E> {
    /// Physical/identity-mapped base address this handle denotes.
    pub addr: nat,
    /// Decoded entry at each valid slot — `entries[i]` is what `read(i)` returns
    /// (`None` = the slot holds a word that is not a valid encoding of `E`).
    pub entries: Map<nat, Option<E>>,
}

impl<E: TableEntry> View for Table<E> {
    type V = TableView<E>;

    closed spec fn view(&self) -> TableView<E> {
        TableView {
            addr: self.base as nat,
            entries: Map::new(
                |i: nat| i < crate::mem::PAGE_TABLE_LENGTH,
                |i: nat| spec_table_read::<E>(self.base as nat, i),
            ),
        }
    }
}

} // verus!
