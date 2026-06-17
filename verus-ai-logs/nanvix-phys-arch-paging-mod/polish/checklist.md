# Polish Report: arch-paging-mod

## Proof Extraction
- Blocks extracted: 0 (none to extract)
  - `check_proof_blocks.py src/libs/arch/src/x86/mem/paging/mod.rs --all` →
    "No proof blocks found. No loop invariants found."
  - Spec file (`mod.spec.rs`) and proof file (`mod.proof.rs`) are empty stubs
    (`verus! { }`); there are no inline `proof { ... }` blocks or loop invariants
    in `mod.rs`.
- Blocks kept inline: 0 (no proof blocks exist)

## Minimization
- Redundant assertions removed: 0 (none present — no `assert` statements in `mod.rs`)
- Redundant lemmas/hints removed: 0 (no lemmas, `by(...)` hints, or spec functions exist)
- Dead spec functions removed: 0 (none present)

## Notes
- The only in-scope function is `invlpg`, a `#[verus_verify(external_body)]`
  whose body is a single `core::arch::asm!` issuing the `invlpg` instruction.
  It is listed in `verus-ai-logs/tcb-allowed.md` and documented in the module's
  `verus-unsupported.md`. Its empty contract (no `requires`/`ensures`) matches the
  inherited upstream `assume_specification[ ::arch::mem::paging::invlpg ]`.
- The trust-boundary comment above `invlpg` is required TCB documentation, not a
  removable verification hint, so it was retained per the proof-minimization skill.
- The module was already in a minimal, integration-ready state; no extraction or
  minimization changes were necessary.

## Verification
- `make verify-arch` → exit 0.
- Cheating check: `assume=0 admit=0` for the crate; `external_body=3` are all
  allow-listed (`mod.rs:80 invlpg`, `table.rs:201 read`, `table.rs:238 write`).
- Verus verifies with 0 errors / 0 admits; build passes.
