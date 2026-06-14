# Cheating Elimination Report: phys-upool

## Scope

Module `mm::phys::upool` — files `upool.rs`, `upool.spec.rs`, `upool.proof.rs`.
Target functions (only functions in scope): `UserFrame::share`,
`UserFrame::refcount`, `Upool::new`, `UserFrame::leak`, `UserFrame::drop`,
`Upool::alloc`, `UserFrame::new`, `UserFrame::address`.

Hard rule observed: unlisted functions (in `frame.rs`, `manager.rs`, `mod.rs`,
`kframe.rs`) and the protected spec/view definitions were **not** modified.

Verification (forced, non-cached): `make verify-kernel MODULE=mm::phys`
→ **42 verified, 0 errors, exit 0**. Full crate `make verify` → exit 0.

## Cheating Counts (before → after) — upool module files only

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 3      | 2*    | 1          |
| assume_specification | 0      | 0     | 0          |
| cfg-gated exec       | 3**    | 3**   | 0          |

\* The two remaining `external_body` (`Upool::new`, `Upool::alloc`) are
authorized in `verus-ai-logs/tcb-allowed.md` **and** proven mathematically
irreducible in this specification phase (Verus errors captured — see
`verification_todo.md`). The frozen-`phys_view()` convention defers their
state transitions to a proving-phase ghost token.

\** Pre-existing, unchanged: two `#[cfg(verus_keep_ghost)]` gates on
`include!("upool.spec.rs")` / `include!("upool.proof.rs")` (compile-time
spec/proof inclusion, not exec logic) and one `#[cfg(not(verus_keep_ghost))]`
guarding an `error!` log line in `UserFrame::drop`. None introduced by this
task; none alter exec semantics, time, or space complexity.

Global crate counts moved 18 → 17 external_body as a result of this phase.

## Items Eliminated

1. **`Upool` (struct) `external_body` → removed.**
   - Was: `#[verus_verify(external_body)]` on `pub struct Upool { _private: () }`.
   - Escalation ladder: the struct is `{ _private: () }`, trivially representable
     by Verus, so the `external_body` trust boundary was unnecessary. Removing it
     and re-verifying gives **42 verified, 0 errors** — the `View for Upool`
     `uninterp` declaration remains valid on a transparent struct, and the
     manager's dependency on `Upool::alloc`'s contract is unaffected (no spec
     changed).
   - Result: now plain `#[verus_verify]`; one fewer `external_body`.

## Items NOT eliminable this phase (genuinely-stuck, documented)

- **`Upool::new` `external_body`** — `ensures result@.wf()` over an
  `uninterp` `View for Upool`; unprovable from the empty struct body.
  Verus error on removal: `postcondition not satisfied (result@.wf())`.
- **`Upool::alloc` `external_body`** — `ensures final(self)@ ==
  old(self)@.alloc_one(uf@)`, a `self@` transition the body's `frame::alloc()`
  call cannot supply (its contract is over the frozen global `phys_view()`, and
  `self` is structurally unchanged). Verus errors on removal: two
  `postcondition not satisfied`.

Both are authorized in `tcb-allowed.md` and recorded in
`verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md`. Eliminating them
requires the `frame` free-function layer's real transitions (separate phase),
i.e. touching `frame.rs`, which the hard rules forbid here.

All eight target functions verify with real contracts (no admit/assume):
`UserFrame::new`/`address`/`leak` from `UserFrame::inv` (page-alignment);
`UserFrame::share`/`refcount` from the `frame` dependency contracts;
`UserFrame::drop` (`opens_invariants none`, `no_unwind`); `Upool::new`/`alloc`
as authorized `external_body` above.

## Verification TODOs

See `verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md`. No `admit`/
`assume`/`no_decreases`/`limitation_assume` remain. The two irreducible
`external_body` are documented with Verus evidence and the proving-phase
condition under which they become provable.

## AST Consistency

- Exec-code diff vs pre-task baseline (`0922a8e0c`): a **single** attribute
  change — `#[verus_verify(external_body)]` → `#[verus_verify]` on the `Upool`
  struct. All other hunks are documentation comments.
- This is a verification-only annotation (`verus_verify`/`external_body` are
  Verus directives). The struct layout, fields, `#[derive(Debug)]`, and all
  runtime behavior are unchanged. **Semantics, time complexity, and space
  complexity preserved.**
- Evidence the change is sound and required-direction (cheating reduction):
  module re-verifies at 42 verified, 0 errors with the struct transparent.
- No `external_body` added; no `cfg`-gated exec workaround introduced; no exec
  signatures changed.
- Zero unjustified mismatches confirmed: **YES**

## Regression

- `make verify-kernel MODULE=mm::phys`: 42 verified, 0 errors (exit 0).
- `make verify` (full crate): exit 0. Crate-wide residual cheating
  (admit=24, external_body=17) is entirely in modules outside this phase's scope
  and unchanged except for the 18→17 `external_body` reduction this phase made.

## Result: PASS

Rationale: the upool module contains zero `admit`/`assume`/`no_decreases`/
`limitation_assume`/`assume_specification`. One unnecessary `external_body` (the
struct) was eliminated. The two remaining `external_body` are authorized in
`tcb-allowed.md` and are proven irreducible in this specification phase (Verus
errors captured), with elimination deferred to the dependent `frame` phase.
Verus verification passes cleanly (exit 0).
