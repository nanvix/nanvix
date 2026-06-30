verus! {

//==================================================================================================
// TableIndex — a validated page-table slot number
//==================================================================================================

/// A `TableIndex` carries one caller-visible value: the validated index.
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

/// Low `log2(PAGE_TABLE_LENGTH)` bits of `vaddr >> shift`.
///
/// Mirrors the exec masking `(vaddr >> shift) & (PAGE_TABLE_LENGTH - 1)` exactly.
pub open spec fn spec_table_index(vaddr: usize, shift: usize) -> nat {
    ((vaddr >> shift) & ((crate::mem::PAGE_TABLE_LENGTH - 1) as usize)) as nat
}

/// PD index: bits `[PGTAB_SHIFT, PGTAB_SHIFT + log2(LEN))` of `vaddr`.
pub open spec fn spec_pd_index(vaddr: usize) -> nat {
    spec_table_index(vaddr, crate::mem::PGTAB_SHIFT)
}

/// PT index: bits `[PAGE_SHIFT, PAGE_SHIFT + log2(LEN))` of `vaddr`.
pub open spec fn spec_pt_index(vaddr: usize) -> nat {
    spec_table_index(vaddr, crate::mem::PAGE_SHIFT)
}

//==================================================================================================
// TableEntry codec (the per-entry round-trip law)
//==================================================================================================

/// Abstract serialization of a `TableEntry`.
pub uninterp spec fn spec_entry_raw<E>(e: E) -> PteWord;

/// Abstract decoding of a `TableEntry`.
///
/// Returns `None` for a word that is not a valid encoding of `E`.
pub uninterp spec fn spec_entry_from_raw<E>(w: PteWord) -> Option<E>;

//==================================================================================================
// Global ghost model of page-table memory
//==================================================================================================

/// Raw word currently stored at slot `index` of the page-table page based at `addr`.
pub uninterp spec fn spec_table_word(addr: nat, index: nat) -> PteWord;

/// The decoded entry currently stored at slot `index` of the page based at `addr`.
pub open spec fn spec_table_read<E>(addr: nat, index: nat) -> Option<E> {
    spec_entry_from_raw::<E>(spec_table_word(addr, index))
}

//==================================================================================================
// Table<E> — a typed, non-owning handle over one page-table page
//==================================================================================================

/// Abstract view of a typed page-table handle.
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
                Set::range(0, crate::mem::PAGE_TABLE_LENGTH as nat),
                |i: nat| spec_table_read::<E>(self.base as nat, i),
            ),
        }
    }
}

} // verus!
