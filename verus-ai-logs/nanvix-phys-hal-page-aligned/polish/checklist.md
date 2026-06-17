# Polish Report: hal-page-aligned

Scope: `src/kernel/src/hal/mem/types/address/aligned/page.rs` (+ `.spec.rs`, `.proof.rs`).
In-scope functions: `PageAligned::into_raw_value`, `PageAligned::from_address`, `PageAligned`.

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py --all` reports "No proof blocks found" and "No loop
    invariants found". The exec file (`page.rs`) contains no inline `proof { ... }`
    blocks and no loop invariants; the proof file (`page.proof.rs`) is empty
    (`verus! { }`). The in-scope functions are trivial delegations
    (`into_raw_value` → `self.0.into_raw_value()`; `from_address` validates via
    `is_aligned` with no proof body), so there is nothing to factor out.
- Blocks kept inline: 0 (none exist).

## Minimization
- Redundant assertions removed: 0
  - No `assert` statements exist in the in-scope code (verified by source scan).
- Redundant lemmas/hints removed: 0
  - No lemmas exist (`page.proof.rs` is empty). No `by(...)` /
    `by(nonlinear_arith)` / trigger hints present.
- Dead spec functions removed: 0
  - `spec_aligned(int)` — kept: used in `from_address`'s `ensures` to express the
    input-side condition on the raw `T` address (the `Err` arm needs
    `!spec_aligned(addr@)`, which cannot use `PageAligned::inv` because `addr: T`
    is not yet a `PageAligned`). Also `pub` API.
  - `PageAligned::inv` — kept: `pub` type-invariant returned by `from_address`
    (`r.inv()`); it is the meaningful page-alignment guarantee downstream callers
    rely on. Removing it would weaken the `ensures`.
  - `View::view` — kept: used throughout the contract (`addr@`, `r@`, `self@`).
  - Both `assume_specification`s (`PAGE_ALIGNMENT`, `Deref::deref`) — kept:
    required trusted specs for external constants/std-trait methods called from
    verified code (documented in `page.spec.rs`).
- Debug artifacts removed: 0
  - No TODO/FIXME comments, commented-out code, or property-ID annotations
    (`// INV-*`, `// *-POST-*`). All comments in `.spec.rs` are design rationale
    and were preserved.

## Result
The module was already in a minimal, readable state from the proving phase;
no extraction or removal was warranted under the proof-extraction /
proof-minimization rules ("when in doubt, keep it"; do not weaken `ensures`).

- Verification: `make verify-kernel MODULE=hal::mem::types::address::aligned::page`
  → exit 0, 11 verified, 0 errors, 0 admits/external_body in this module.
- Build: `make all-kernel` → `Finished`, `kernel.elf` produced.
