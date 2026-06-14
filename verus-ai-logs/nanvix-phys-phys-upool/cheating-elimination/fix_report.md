# Cheating Elimination Report: phys-upool

## Scope

Module `mm::phys::upool` — files `upool.rs`, `upool.spec.rs`, `upool.proof.rs`.
Target functions: `UserFrame::share`, `UserFrame::refcount`, `Upool::new`,
`UserFrame::leak`, `UserFrame::drop`, `Upool::alloc`, `UserFrame::new`,
`UserFrame::address`.

Verification (forced, non-cached): `make verify-kernel MODULE=mm::phys`
→ **42 verified, 0 errors, exit 0**.

## Cheating Counts (before → after)

Counts below are scoped to the `upool` module files only.

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 3*     | 3*    | 0          |
| assume_specification | 0      | 0     | 0          |
| cfg-gated exec       | 3**    | 3**   | 0          |

\* All 3 `external_body` are explicitly authorized in
`verus-ai-logs/tcb-allowed.md` (the `Upool` struct, `Upool::new`,
`Upool::alloc`). They are permitted exceptions, not blockers.

\** Pre-existing, unchanged from base (`verus-ai-prove`): two
`#[cfg(verus_keep_ghost)]` gates on `include!("upool.spec.rs")` /
`include!("upool.proof.rs")` (compile-time spec/proof inclusion, not exec
logic) and one `#[cfg(not(verus_keep_ghost))]` guarding an `error!` log line
in `UserFrame::drop`. None introduced by this task; none alter exec semantics,
time, or space complexity.

## Items Eliminated

None required. On entry, the `upool` module already contained **zero**
unauthorized cheating:

- No `admit()` / `assume()` in `upool.rs`, `upool.spec.rs`, or
  `upool.proof.rs` (the proof file is empty: `verus! { }`).
- The only `external_body` are the three authorized entries in
  `tcb-allowed.md`. Each is structurally impossible to verify in-body because
  `View for Upool` is `uninterp` (`uninterp spec fn view(&self) ->
  FrameAllocView;`), so no body can establish post-state facts (`result@.wf()`,
  `alloc_one(..)`) about an uninterpreted view. Removing the `external_body`
  would require redefining the view (out of scope / not a target function) and
  threading ghost state from the global frame allocator — exactly the deferral
  the TCB entry records.

All eight target functions verify with real contracts:

- `UserFrame::new` / `address` / `leak` — discharged from `UserFrame::inv`
  (page-alignment of `self@`) preserved through `ManuallyDrop` / field copy.
- `UserFrame::share` / `refcount` — delegate to the (TCB-authorized) `frame`
  free functions whose contracts supply the `phys_view().frames` post-state
  facts; the `match`/`Ok`/`Err` postconditions follow.
- `UserFrame::drop` — `opens_invariants none`, `no_unwind`; best-effort
  `frame::free`.
- `Upool::new` / `Upool::alloc` — authorized `external_body` (see above).

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md)

None. No remaining proof gaps. The file documents that the three
`external_body` are authorized TCB exceptions (uninterp `View for Upool`), not
unfinished proofs.

## AST Consistency

- No exec-code changes were made to `upool.rs` (`git diff verus-ai-prove --
  src/kernel/src/mm/phys/upool*` is empty). Files are byte-identical to base.
- No `external_body` added; no `cfg`-gated exec workarounds introduced; no
  signatures changed. Semantics, time complexity, and space complexity are
  unchanged.
- Zero mismatches confirmed: **YES**

## Regression

- `make verify-kernel MODULE=mm::phys`: 42 verified, 0 errors (exit 0).
- `make verify` (full crate): exit 0, verification passes. Crate-wide cheating
  counts (admit=24, external_body=18) are entirely in modules outside this
  task's scope and are unchanged from base.

## Result: PASS
