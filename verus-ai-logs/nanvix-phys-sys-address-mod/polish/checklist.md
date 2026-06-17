# Polish Report: sys-address-mod

In-scope file: `src/libs/sys/src/sys/mm/address/mod.rs` (+ `mod.spec.rs`, `mod.proof.rs`).
In-scope functions: `is_aligned`, `into_raw_value`, `from_raw_value`.

All three target functions are **trait method declarations** on the `Address`
trait. They carry `#[verus_spec]` contracts but have no executable bodies — the
bodies live in concrete implementors (out-of-scope `virt` module). A bodyless
trait declaration has no proof obligation, so there are no inline proof blocks,
loop invariants, assertions, or proof hints to polish. `mod.proof.rs` is empty.

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py src/libs/sys/src/sys/mm/address/mod.rs --all`
    reports: "No proof blocks found. No loop invariants found."
  - No `proof { ... }` blocks, `assert`, or `by(...)` hints exist in any
    in-scope file, so there was nothing to extract.
- Blocks kept inline: 0 (none exist).

## Minimization
- Redundant assertions removed: 0 (none exist in the in-scope files).
- Redundant lemmas/hints removed: 0 (`mod.proof.rs` is empty; no `by(...)`
  hints or lemmas exist).
- Dead spec functions removed: 0.
  - `mod.spec.rs` defines a single spec fn, `spec_addr_is_aligned`, which is
    **not** dead: it is referenced by `is_aligned`'s `ensures`
    (`mod.rs:138`). It is also `pub open`, which the proof-minimization skill
    requires keeping as part of the module's API for dependents.
- Debug artifacts removed: 0. The comment in `mod.spec.rs` is explanatory
  documentation, not a property-ID tag / TODO / commented-out code, so it is
  retained.

## Verification
- `make verify-sys` → exit 0, status **CLEAN**.
- Cheating scan: `assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
- 0 errors, 0 admits. No spec weakening (no `requires`/`ensures` changed).

## Conclusion
The module is already minimal and integration-ready. No edits were required:
no proof blocks to extract and no redundant proof artifacts to remove without
weakening guarantees or violating source-integrity constraints.
