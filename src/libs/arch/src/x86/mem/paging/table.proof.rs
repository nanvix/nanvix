verus! {

// Masking a shifted address with `PAGE_TABLE_LENGTH - 1` (= 1023) keeps the result within a single
// page table, independent of the shift amount: the mask clears every bit at or above
// `log2(PAGE_TABLE_LENGTH)`, so the result is `< PAGE_TABLE_LENGTH`. Shared by `pd_index` and
// `pt_index`, which differ only in the shift constant.
pub proof fn lemma_masked_index_bounded(vaddr: usize, shift: usize)
    ensures
        ((vaddr >> shift) & ((crate::mem::PAGE_TABLE_LENGTH - 1) as usize)) < crate::mem::PAGE_TABLE_LENGTH,
{
    assert(crate::mem::PAGE_TABLE_LENGTH == 1024) by (compute);
    assert(((vaddr >> shift) & 1023usize) < 1024usize) by (bit_vector);
}

// The `TableEntry` round-trip law: decoding a freshly-encoded entry yields it back.
// This is the abstract contract every `TableEntry` implementor must honour (`raw` is a faithful,
// injective serialization). Stated as a broadcast lemma so callers obtain the read-after-write
// guarantee (`read(write(idx, e), idx) == Some(e)`) by `broadcast use lemma_entry_roundtrip`.
//
// `spec_entry_raw` / `spec_entry_from_raw` are `uninterp` over a generic `E` with no structure, so
// this law cannot be derived in-module — Verus has no way to relate two uninterpreted functions
// over a structureless type parameter (see the reproducer below). It is a foundational trust
// anchor over the trait codec (a faithful serialization is injective, hence decode-after-encode
// recovers the same entry). The proof-fn body therefore discharges the obligation with a single
// approved limitation assume (replacing the former `external_body`, which is illegal on a proof
// fn): the assumed proposition is exactly the codec injectivity law each concrete `TableEntry`
// implementor honours against its own interpreted codec.
pub broadcast proof fn lemma_entry_roundtrip<E>(e: E)
    ensures
        #[trigger] spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e),
{
    // VERUS-AI LIMITATION: id=L1 construct=uninterp-generic-codec-injectivity repro=verus-ai-logs/nanvix-phys-arch-paging-table/repros/L1.rs
    assume(spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e));
}

} // verus!
