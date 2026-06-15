verus! {

// The `TableEntry` round-trip law: decoding a freshly-encoded entry yields it back.
// This is the abstract contract every `TableEntry` implementor must honour (`raw` is a faithful,
// injective serialization). Stated as a broadcast lemma so callers obtain the read-after-write
// guarantee (`read(write(idx, e), idx) == Some(e)`) by `broadcast use lemma_entry_roundtrip`.
//
// `spec_entry_raw` / `spec_entry_from_raw` are `uninterp` over a generic `E` with no structure,
// so this law cannot be derived in-module: it is a foundational trust anchor over the trait codec
// (a faithful serialization is injective, hence decode-after-encode recovers the same entry). It
// is therefore a trusted broadcast axiom (`external_body`, no body), recorded in
// `verus-ai-logs/tcb-allowed.md`. Each concrete `TableEntry` implementor discharges the same law
// against its own (interpreted) codec. This replaces the spec-phase `admit()` placeholder with the
// idiomatic Verus axiom form — no behavioral change to the trusted contract.
#[verifier::external_body]
pub broadcast proof fn lemma_entry_roundtrip<E>(e: E)
    ensures
        #[trigger] spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e),
{
}

} // verus!
