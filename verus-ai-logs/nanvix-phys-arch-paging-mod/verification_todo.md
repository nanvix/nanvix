# Verification TODOs — arch-paging-mod

## Deferred to the `table` proving phase (no admit/assume remains)

### `TableEntry` round-trip law (formerly `table.proof.rs::lemma_entry_roundtrip`)

- **What:** the broadcast law
  `spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e)`.
- **Why it could not be proven in place:** `spec_entry_raw` / `spec_entry_from_raw`
  are `uninterp spec fn`, and the lemma is generic over `E` with **no** `TableEntry`
  bound, so the body has zero information about the codec — the proposition is a pure
  axiom. A genuine proof requires per-implementor, bit-level reasoning over the
  concrete `PageDirectoryEntry` / `PageTableEntry` encodings (8 flag codecs +
  `FrameNumber` shift in `pde.rs` / `pte.rs` / `flags.rs`), **none of which carry
  `#[verus_spec]` contracts yet**. That is the core of the not-yet-run `table`
  proving phase (only its specification dialogue + a crash log exist under
  `verus-ai-logs/nanvix-phys-arch-paging-table/`).
- **Resolution applied now:** the law was an undischarged spec-phase placeholder
  whose body was an unproven axiom. It is **never `broadcast use`d** anywhere in the
  crate (verified repo-wide), so no proof depends on it. To reach zero cheating
  without leaving an axiom, the dead placeholder was **removed** (it is not replaced
  by `assume`/`external_body`, which would also be cheating).
- **Recommended proper fix (table proving phase):** add a `proof fn` obligation to the
  `TableEntry` trait
  (`proof fn lemma_roundtrip(e: Self) ensures spec_entry_from_raw::<Self>(spec_entry_raw(e)) == Some(e)`),
  discharge it in each implementor by giving `from_raw_value` / `into_raw_value`
  (and the flag/frame codecs) real `#[verus_spec]` contracts and proving the bit-level
  round-trip, then re-export
  `lemma_entry_roundtrip<E: TableEntry>(e: E) { E::lemma_roundtrip(e); }` as the
  broadcast lemma for the read-after-write guarantee.

## In scope for this phase (`invlpg`)

- None. `invlpg`'s only cheating construct is `#[verus_verify(external_body)]`, which
  is on the TCB allowlist (`verus-ai-logs/tcb-allowed.md`) as an external-bottom
  hardware boundary (inline `asm!`, unsupported by Verus). No proof gap.
