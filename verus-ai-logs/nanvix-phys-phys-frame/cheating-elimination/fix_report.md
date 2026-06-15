# Cheating Elimination Report: phys-frame

Scope: `src/kernel/src/mm/phys/frame.rs` (+ `frame.spec.rs` / `frame.proof.rs`).
Module verification: `make verify-kernel MODULE=mm::phys`.
Baseline commit: `f884ee245`. Allowlist: `verus-ai-logs/tcb-allowed.md`.

## Cheating Counts (before → after) — frame.rs / frame.spec.rs / frame.proof.rs

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 8      | 7     | 1          |
| assume_specification | 2      | 2     | 0          |
| cfg-gated exec       | 23     | 23    | 0          |

Notes:
- The 7 remaining `external_body` are **all** listed in `tcb-allowed.md`
  (`instance`, `init`, `alloc`, `alloc_contiguous`, `free`, `book`, `alloc_range`),
  so none is a cheating-gate blocker.
- `assume_specification` (2: `PageAligned::into_raw_value`, `PageAligned::deref`) are
  external-bottom trusted contracts for the not-yet-verified `hal::mem` address layer,
  listed in `tcb-allowed.md`; superseded when that layer is verified.
- `cfg-gated exec` (23) are pre-existing, sanctioned `#[cfg(not(verus_keep_ghost))]` /
  `#[cfg_attr(verus_keep_ghost, verus_spec(...))]` gates over Verus-unsupported exec
  constructs (`debug_assert!`, `error!`/`info!` logging macros, exec loop-invariant
  attachment). They preserve exec semantics (AST-consistency MATCH) and are the
  verus-constraints-approved handling, not cheating to remove. Net new gates introduced: 0.

Whole-module scan (`make verify-kernel MODULE=mm::phys`) still reports cheating in *other*
files of `mm::phys` (`manager`, `mod`, `upool`, `kframe`); those are outside the phys-frame
scope and untouched.

## Items Eliminated

- **`free_count` — `external_body` removed, body verified.**
  - What it was: `#[verus_verify(external_body)]` wrapper returning
    `number_of_bits() - usage()`, with `ensures result as nat == phys_view().frames.free_count()`.
  - How eliminated: added proof lemma `lemma_free_count_eq` (frame.proof.rs) proving, from
    `inner.inv()`, that `free_frames.len() == num_bits - set_bits.len()`:
    - free-index set `idx = [0, num_bits) \ set_bits`; `set_bits ⊆ [0,num_bits)` (BitmapView
      `wf()`), so `|idx| = num_bits − |set_bits|` via `vstd::set_lib::lemma_int_range` +
      `lemma_set_disjoint_lens`;
    - `free_frames == idx.map(frame_addr_of)` (extensional, witnessed both directions);
    - `frame_addr_of` (`i ↦ i·PAGE_SIZE`) injective on integers (`PAGE_SIZE > 0`,
      `by(nonlinear_arith)`), so `vstd::set_lib::lemma_map_size` gives
      `free_frames.len() == idx.len()`.
    - Set finiteness (`set_bits.finite()`) recovered through the bitmap crate's public
      `BitmapView::lemma_set_bits_finite` (its `internal_inv` is crate-`closed`); `num_bits > 0`
      taken from the exec `number_of_bits()` postcondition.
  - Escalation ladder followed: searched vstd (`set_lib`: `lemma_int_range`,
    `lemma_set_disjoint_lens`, `lemma_map_size`, `lemma_set_subset_finite`; `relations::injective_on`)
    rather than inventing axioms; no `assume`/`admit` used.
  - Result: `make verify-kernel MODULE=mm::phys` → 85 verified, 0 errors; full
    `make verify-kernel` → 115 verified, 0 errors. Global `external_body` 17 → 16.

## Verification TODOs (`verus-ai-logs/nanvix-phys-phys-frame/verification_todo.md`)

Genuinely-stuck wrappers (all `external_body`, all tcb-allowed); each blocker reproduced by
removing `external_body` and re-running module verification:

- `alloc` — `Ok` postcondition `phys_view().frames.allocated_frames.contains(frame@)` is a
  **post-mutation** fact; `instance()` pins only the pre-call `phys_view().frames`
  (`postcondition not satisfied`). Needs §8 singleton ghost token.
- `book` — `Ok` postcondition `phys_view().frames.reserved(phys_addr@)`; same post-mutation
  reference (`postcondition not satisfied`). Needs §8 singleton ghost token.
- `alloc_range` — `Ok` postcondition `phys_view().frames.all_reserved(region_frame_addrs(...))`;
  same post-mutation reference (`postcondition not satisfied`). Needs §8 singleton ghost token.
- `alloc_contiguous` — `Ok` postcondition `base@ + count·PAGE_SIZE <= usize::MAX` is the
  one-past-the-end address `frame_addr_of(lo+count)`, not bounded by `internal_inv` (which
  bounds only indices `< num_bits`) when the range ends at `num_bits`
  (`postcondition not satisfied`). Needs a strengthened allocator invariant
  (`num_bits·PAGE_SIZE ≤ usize::MAX`) bridged in the proving phase.
- `free` — contract is `opens_invariants none` + `no_unwind`, but `instance()` may open
  invariants and panics when uninitialized (`callee may open invariants that caller cannot`;
  `cannot show this call will not unwind`). Needs a `no_unwind`/`opens_invariants none`
  singleton accessor.

## AST Consistency

- Tool: `scripts/ast_consistency.py` (tree-sitter exec hashing) against base
  `verus-ai-prove-bottom-up:src/kernel/src/mm/phys/frame.rs`.
- Result: **Consistent: YES (matched=19, mismatched=0, missing=0, extra=0).**
- The only exec touch is in `free_count` (bind the two bitmap reads to locals so the proof can
  observe `number_of_bits() > 0` before the lemma) — a pre-approved *intermediate value*
  deviation, semantically identical (same calls, order, result); the checker reports MATCH.
- Zero mismatches confirmed: **YES**.

## Result: PASS

- 0 `admit` / 0 `assume` in scope.
- 1 `external_body` eliminated (`free_count`) with a real proof; the 7 remaining are all in
  `tcb-allowed.md` (no non-allowed `external_body`).
- `make verify-kernel` → 115 verified, 0 errors; `make verify` → all crates pass, 0
  verification errors (no regressions).
- AST consistency: 0 mismatches.
