verus! {

// The `TableEntry` round-trip law: decoding a freshly-encoded entry yields it back.
// This is the abstract contract every `TableEntry` implementor must honour (`raw` is a faithful
// serialization). Stated as a broadcast lemma so callers obtain the read-after-write guarantee
// (`read(write(idx, e), idx) == Some(e)`) by `broadcast use lemma_entry_roundtrip`. Body is
// `admit()` during the specification phase; the proving phase discharges it per implementor.
pub broadcast proof fn lemma_entry_roundtrip<E>(e: E)
    ensures
        #[trigger] spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e),
{
    admit();
}

} // verus!
