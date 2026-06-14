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
// Table<E> — a typed, non-owning handle over one page-table page
//==================================================================================================

// To callers a `Table<E>` is a typed handle over the page at a physical/identity-mapped base
// address (see `view_design.md`). The caller-meaningful identity is *which page* read/write act
// on. The per-slot entry map (`index -> Option<E>`) is volatile, caller-owned memory that the
// struct does not store, so it is not part of this struct-level view; per `view_design.md`'s Open
// Mechanism Note it is realized by a memory-permission token in the proving phase.
pub struct TableView {
    /// Physical/identity-mapped base address this handle denotes.
    pub addr: nat,
}

impl<E: TableEntry> View for Table<E> {
    type V = TableView;

    closed spec fn view(&self) -> TableView {
        TableView { addr: self.base as nat }
    }
}

impl TableView {
    // A page-table page lives entirely inside the address space: its base plus one page does not
    // wrap `usize`. This is what lets `read`/`write` form `base + index * 4` without overflow and
    // keeps every access inside `[addr, addr + PAGE_SIZE)`.
    pub open spec fn inv(self) -> bool {
        self.addr + crate::mem::PAGE_SIZE <= usize::MAX
    }
}

impl<E: TableEntry> Table<E> {
    #[verifier::type_invariant]
    pub open spec fn inv(&self) -> bool {
        self@.inv()
    }
}

} // verus!
