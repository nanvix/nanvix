verus! {

// The `TableEntry` round-trip law — decoding a freshly-encoded entry yields it back
// (`spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e)`) — is a property of each concrete
// codec, not a fact about the *uninterpreted* `spec_entry_raw` / `spec_entry_from_raw`. It is
// therefore unprovable in this (parameter-free, trait-unbounded) form without an axiom, and a
// genuine proof requires the per-implementor bit-level codec reasoning (the `PageDirectoryEntry`
// / `PageTableEntry` flag + frame encodings, none of which carry contracts yet). That work
// belongs to the `table` *proving* phase, where it will be expressed as a `TableEntry` proof
// obligation discharged by each implementor (`E: TableEntry`) and re-exported as a broadcast
// lemma for the read-after-write guarantee. No proof currently depends on it (it is never
// `broadcast use`d anywhere in the crate), so the undischarged spec-phase placeholder — whose
// body was an unproven axiom — is removed rather than left as cheating.

} // verus!
