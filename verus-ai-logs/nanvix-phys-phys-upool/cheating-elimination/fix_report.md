# Cheating Elimination Report: phys-upool

## Scope & hard rules

In-scope functions (the only ones for this module): `UserFrame::share`,
`UserFrame::refcount`, `Upool::new`, `UserFrame::leak`, `UserFrame::drop`,
`Upool::alloc`, `UserFrame::new`, `UserFrame::address`. Files: `upool.rs`,
`upool.spec.rs`, `upool.proof.rs`.

Hard rule honored: **unlisted functions were not touched** — in particular
`frame.rs`, `manager.rs`, `manager.proof.rs`, `mod.rs`, `kframe.rs` are
untouched. Protected spec/view definitions were not modified.

Verus: `make verify-kernel MODULE=mm::phys` → **42 verified, 0 errors, exit 0**.

## Cheating Counts (before → after)

### upool's own files (`upool.rs` / `.spec.rs` / `.proof.rs`)

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 3      | 2*    | 1          |
| assume_specification | 0      | 0     | 0          |
| no_decreases (R20p)  | 0      | 0     | 0          |
| limitation_assume    | 0      | 0     | 0          |
| cfg-gated exec       | 3**    | 3**   | 0          |

\* The 2 remaining are exec-fn `external_body` on `Upool::new` / `Upool::alloc`:
listed in `tcb-allowed.md`, mandated by `view_design.md` §8 (`uninterp +
external_body`), proven irreducible (Verus errors captured in
`verification_todo.md`), and **excluded from the hard-cheating gate** by
`verus-ai/workflow.py:_elimination_hard_cheating` (which ignores exec-fn EB).

\** Pre-existing compile-time `#[cfg(verus_keep_ghost)]` gates on the two
`include!()`s plus one `#[cfg(not(verus_keep_ghost))]` on an `error!` log line in
`UserFrame::drop`. Not exec logic; unchanged; semantics/time/space preserved.

### Whole-directory gate scan (`mm/phys/*.rs`, what the harness counts)

| Item          | Before | After | Note |
|---------------|--------|-------|------|
| external_body | 17     | 16    | upool struct EB removed (18→17 global) |
| admit()       | 12     | 12    | 100% in `frame.rs`(8) + `manager.proof.rs`(4) — out of scope |

## Items Eliminated

1. **`Upool` (struct) `external_body` → removed** (now plain `#[verus_verify]`).
   The struct `{ _private: () }` is trivially modeled by Verus; the trust
   boundary was unnecessary. Re-verified: 42 verified, 0 errors. Module EB 3→2.

## Why the gate still reports CHEATING_DETECTED (root cause)

The cheating gate scans the **entire** `src/kernel/src/mm/phys/` directory
(`config.py:source_dir()` → `workflow.py:372 source_dir.glob("*.rs")`), not just
upool. The hard-cheating trigger is `admit_count > 0`. **All 12 admits live in
`frame.rs` and `manager.proof.rs`** — sibling files owned by the separate
`phys-frame` and `phys-manager` phases:

- `frame.rs` (8): `proof! { admit(); }` placeholders in `Inner::alloc/
  alloc_contiguous/free/share/refcount/book/is_covered/alloc_range` — the
  bitmap→`FrameAllocView` transition proofs.
- `manager.proof.rs` (4): `lemma_manager_attached`, `lemma_kernel_alloc_one`,
  `lemma_kernel_alloc_contiguous`, `lemma_user_bulk_err_restored` — the §8
  ghost-token attachment lemmas.

These are **unlisted functions**; touching them violates the task's hard rule and
requires entire dependent phases. No change within `phys-upool`'s scope can
reduce this count. upool's own contribution to hard-cheating is **zero**.

## Verification TODOs

See `verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md`. upool's files
carry no `admit`/`assume`/`trusted`/`no_decreases`/`limitation_assume`. The two
exec `external_body` are documented with Verus evidence; the 12 hard-cheating
admits are mapped to their owning out-of-scope phases.

## AST Consistency

- Exec-code diff vs pre-task baseline (`0922a8e0c`): a **single** attribute
  change — `#[verus_verify(external_body)]` → `#[verus_verify]` on the `Upool`
  struct. Everything else is documentation comments.
- Verification-only annotation; struct layout, fields, `#[derive(Debug)]`, and
  all runtime behavior unchanged. **Semantics, time, and space complexity
  preserved.** Evidence it is sound: module re-verifies at 42 verified, 0 errors.
- No `external_body` added; no `cfg`-gated exec workaround introduced; no exec
  signatures changed.
- Zero unjustified mismatches: **YES**

## Regression

- `make verify-kernel MODULE=mm::phys`: 42 verified, 0 errors (exit 0).
- `make verify` (full crate, prior run this session): exit 0.

## Result: PASS (within scope) — sibling-file admits are out-of-scope BLOCKERS for other phases

Within `phys-upool`'s permitted scope, all cheating is eliminated or
authorized: upool's files have zero hard-cheating, one unnecessary
`external_body` was removed, and the two remaining exec `external_body` are
tcb-allowed, design-mandated, irreducible, and excluded from the hard-cheating
gate. The residual 12 admits that keep the directory-scoped gate at
CHEATING_DETECTED are entirely in `frame.rs` / `manager.proof.rs`, which the
hard rule forbids this phase from modifying; they belong to the `phys-frame` and
`phys-manager` phases.
