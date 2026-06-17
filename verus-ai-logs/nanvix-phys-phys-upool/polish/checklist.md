# Polish Report: phys-upool

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py upool.rs --all` reports "No proof blocks found"
    and "No loop invariants found". The in-scope exec functions
    (`UserFrame::share`, `refcount`, `new`, `address`, `leak`, `drop`,
    `Upool::new`, `Upool::alloc`) contain no inline `proof { ... }` blocks.
- Blocks kept inline: 0 (none exist)

## Minimization
- Redundant assertions removed: 0
  - No `assert` / `by(...)` / `reveal` / lemma-call artifacts exist in
    `upool.rs`, `upool.spec.rs`, or `upool.proof.rs`.
- Redundant lemmas/hints removed: 0
  - `upool.proof.rs` is already empty (`verus! { }`).
  - `upool.spec.rs` defines only `UserFrame::inv`, which is `pub open spec`
    and referenced by the `requires`/`ensures` of every in-scope method, so it
    is not dead and is retained.
  - The `external_body` rationale comments on `Upool::new` / `Upool::alloc`
    document the trust boundary (mirroring `tcb-allowed.md` entries 251/289);
    they are meaningful design documentation, not obsolete proof hints or
    property-ID tags, so they are retained per the proof-minimization rules.

## Verification
- `make verify-kernel MODULE=mm::phys` → exit code 0 (verification passes).
- upool scope: 0 admits, 0 errors. The only cheating-detail entries for upool
  are the two TCB-allowed `external_body` functions (`upool.rs:251 new`,
  `upool.rs:289 alloc`). The global `CHEATING_DETECTED` aggregate originates
  from out-of-scope modules (`manager.proof.rs` admits, `frame.rs`, etc.).

## Outcome
The phys-upool module was already in a fully polished state: no inline proof
blocks to extract and no redundant assertions, hints, lemmas, or dead spec
functions to remove. No source/spec/proof changes were required; verification
remains green.
