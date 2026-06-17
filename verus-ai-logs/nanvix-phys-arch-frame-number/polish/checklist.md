# Polish Report: arch-frame-number

## Proof Extraction
- Blocks extracted: 0
- Blocks kept inline: 1 (each ≤ 5 lines or single assert)
  - `FrameNumber::into_raw_value` @ number.rs:100 — `proof! { use_type_invariant(self); }`
    (1 line, single statement). Required: it exposes the type invariant
    `0 <= self@ <= Self::spec_max()` so the postcondition is discharged. Confirmed
    by removal test (postcondition fails: 47 verified, 1 error). Kept inline per the
    proof-extraction rule (≤ 5 lines stay inline).

`check_proof_blocks` result: 1 proof block total, 0 over 5 lines.

## Minimization
- Redundant assertions removed: 0
  - No Verus `assert`/`assert by` statements exist in the in-scope verified functions.
    (The `assert_eq!`/`assert!` in number.rs are runtime Rust assertions inside
    `#[test]` functions — out of scope and not verification artifacts.)
- Redundant lemmas/hints removed: 0
  - Proof file (`number.proof.rs`) is empty; no lemmas to deduplicate.
  - The single proof hint (`use_type_invariant`) is necessary, not redundant
    (verified by removal test above), so it was retained.
- Dead spec functions removed: 0
  - `view` (View trait impl, `closed`), `spec_max` (`pub`), and `inv`
    (`pub`, `#[verifier::type_invariant]`) are all live and part of the type's
    public verification surface. `pub` spec functions are preserved as API per skill.
- Debug artifacts removed: 0
  - No TODO/FIXME comments, commented-out code, or property-ID annotations
    (e.g. `// INV-2`, `// FUNC-POST-3`) present in the source/spec/proof files.
    Existing spec comments are design rationale / kernel cross-references and were kept.

## Verification Status
- `make verify-arch`: **48 verified, 0 errors** (exit 0).
- Cheating check on in-scope file `number.rs`: assume=0, admit=0, external_body=0.
  (The crate-level `external_body=3` / `assume=1` originate from out-of-scope files:
  `x86/mem/paging/mod.rs`, `x86/mem/paging/table.rs`, `x86/mem/paging/table.proof.rs`.)

## Summary
The module was already in an integration-clean state: there are no inline proof
blocks larger than 5 lines to extract, and no redundant assertions, lemmas, hints,
dead spec functions, or debug artifacts to remove. Net source change is zero
(files byte-identical to the verified baseline); verification continues to pass.
