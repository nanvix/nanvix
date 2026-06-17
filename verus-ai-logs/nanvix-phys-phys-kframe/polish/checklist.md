# Polish Report: phys-kframe

Scope: `src/kernel/src/mm/phys/kframe.rs` (+ `.spec.rs` / `.proof.rs`).
In-scope functions: `KernelFrame::new`, `KernelFrame::drop`, `KernelFrame::base`.

Verification: `make verify-kernel MODULE=mm::phys` → exit 0. The kframe module
has **0 errors and 0 admits**. (The 4 admits reported globally are all in the
out-of-scope `manager.proof.rs`; the only kframe TCB item is `kframe.rs:94 new:
external_body`, which is explicitly allowed in `verus-ai-logs/tcb-allowed.md`.)

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py src/kernel/src/mm/phys/kframe.rs --all` → "No proof
    blocks found." `kframe.proof.rs` is empty (`verus! { }`).
  - No `proof { ... }` blocks, loop invariants, or `assert`/`by(...)` hints exist
    in any of the three kframe files.
- Blocks kept inline: 0 (none exist)
  - `new` is `external_body` (allowed) with no proof body.
  - `base` is a pure accessor (`self.base`) with no proof body.
  - `drop` is `opens_invariants none` + `no_unwind` with no proof body.

## Minimization
- Redundant assertions removed: 0 (no `assert` statements exist in scope).
- Redundant lemmas/hints removed: 0 (proof file empty; no `by`/`assume`/`admit`
  hints in scope).
- Dead spec functions removed: 0. The only spec fn, `KernelFrame::inv`, is
  `pub` (module API) and is actively referenced by the `new` and `base`
  contracts (`base.inv()`, `kf.inv()`, `self.inv()`, `result.inv()`); it is not
  dead and is kept per the minimization rule for `pub` spec functions.
- Debug artifacts removed: 0. No TODO/FIXME, commented-out code, or property-ID
  annotations (`// INV-*`, `// FUNC-POST-*`) are present. The contract-rationale
  comments on `new`/`base`/`drop` are substantive integration documentation
  (not obsolete proof hints or property-ID tags) and are kept.

## Conclusion
The kframe verification was already in minimal, extracted form. No inline proof
blocks required extraction and no redundant proof artifacts were found. No source
changes were necessary; verification remains green with 0 errors / 0 admits for
the in-scope module.
