# Polish Report: hal-platform-microvm

Scope: `gva_to_gpa` (the only in-scope function for this module).

Module-scoped verification: `make verify-kernel MODULE=hal::platform::microvm`
→ status **CLEAN**, exit 0 (assume=0, admit=0 in module, no errors).
(The crate-level summary's `admit=4`/`external_body=23` originate from
out-of-scope/cfg-gated functions in the wider kernel crate; none reside in
`microvm/` and none are touched.)

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py src/kernel/src/hal/platform/microvm/mod.rs --all`
    reports: "No proof blocks found. No loop invariants found."
  - `gva_to_gpa` body is a single `gva` return with no inline `proof { ... }`
    block, no loop, and no loop invariants — nothing to extract.
- Blocks kept inline: 0 (none exist).

## Minimization
- Redundant assertions removed: 0
  - No `assert`/`by(...)`/proof hints exist in `gva_to_gpa` (the only `assert`
    token in the file is the unrelated `static_assert::assert_eq_align!` macro,
    out of scope).
- Redundant lemmas/hints removed: 0
  - Proof file `mod.proof.rs` is empty (`verus! { } // verus!`); no lemmas exist.
- Dead spec functions removed: 0
  - `spec_gva_to_gpa` is `pub open spec fn` and is referenced by the `ensures`
    of `gva_to_gpa`; per proof-minimization it is part of the module API and is
    retained.
- Debug artifacts removed: 0
  - No TODO/FIXME, commented-out code, or property-ID annotations in
    `mod.spec.rs`/`mod.proof.rs`; existing comments are genuine documentation
    of the identity-map contract and are retained.

## Conclusion
The in-scope verification was already minimal and free of inline proof blocks.
No source/spec/proof changes were required. Verification passes with 0
errors/admits in scope and the module status is CLEAN.
