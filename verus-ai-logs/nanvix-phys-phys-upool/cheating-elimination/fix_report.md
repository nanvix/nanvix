# Cheating Elimination Report: phys-upool

## Scope

Module under verification: `src/kernel/src/mm/phys/upool.rs` (+ `upool.spec.rs`,
`upool.proof.rs`). In-scope functions: `UserFrame::share`, `UserFrame::refcount`,
`Upool::new`, `UserFrame::leak`, `UserFrame::drop`, `Upool::alloc`, `UserFrame::new`,
`UserFrame::address`.

## Cheating Counts (before → after)

Counts below are scoped to the phys-upool module (source + spec + proof files).

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 3      | 3*    | 0          |
| assume_specification | 0      | 0     | 0          |
| cfg-gated exec       | 0      | 0     | 0          |

\* All 3 remaining `external_body` are explicitly enumerated in
`verus-ai-logs/tcb-allowed.md` and are therefore permitted (design-forced thin-facade
trust boundaries over the global frame allocator — see "Allowed items" below).

## Items Eliminated

None required. The phys-upool module contains **no** disallowed cheating:

- No `admit()`, `assume()`, `assume_specification`, or cfg-gated exec code exists in
  `upool.rs`, `upool.spec.rs`, or `upool.proof.rs`.
- The proof file (`upool.proof.rs`) is empty (`verus! { }`); the spec file defines only
  `UserFrame::inv` (an allowed module spec, not cheating).
- All in-scope verified functions (`UserFrame::new`, `address`, `leak`, `share`,
  `refcount`, `drop`) carry full `#[verus_spec]` contracts and are machine-verified
  (no `external_body`, no proof gaps).

## Allowed `external_body` (per `verus-ai-logs/tcb-allowed.md`)

These are the only 3 cheating items in scope, all sanctioned by the TCB list as permanent
thin-facade boundaries over the global frame allocator (the same wording class as the
`frame.rs` singleton wrappers):

| Location                | Function          | TCB justification (verbatim from tcb-allowed.md)                                                                 |
|-------------------------|-------------------|------------------------------------------------------------------------------------------------------------------|
| `upool.rs:221`          | `Upool` (struct)  | Opaque type; `View` is `uninterp spec fn view() -> FrameAllocView` (no spec-readable field; backing store is the global allocator). |
| `upool.rs:246`          | `Upool::new`      | `ensures result@.wf()` over an uninterpreted view → unprovable; assumed §8 ghost-attachment axiom.                |
| `upool.rs:279`          | `Upool::alloc`    | Delegates to `frame::alloc` (itself `external_body`); `self@`→`phys_view().frames` bridge is the deferred §8 ghost token in the frame free-function layer. |

No `external_body` exists on any in-scope function that is NOT in `tcb-allowed.md`.

## Verification TODOs (`verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md`)

None. There are zero genuine proof gaps in scope; no `verification_todo.md` was created.

## AST Consistency

- Zero mismatches confirmed: **YES**
- `git diff verus-ai-prove-bottom-up -- src/kernel/src/mm/phys/upool.rs
  upool.spec.rs upool.proof.rs` is empty: the module is byte-identical to base. No exec
  code, signatures, or cfg gates were changed, so semantics / time / space complexity are
  trivially preserved.

## Verification Result

- `make verify-kernel MODULE=mm::phys` → exit 0 (verification passes; cached).
- `make verify` (full crate) → exit 0 (no regressions).
- Module-scoped cheating: `assume=0`, `admit=0`, disallowed `external_body=0`,
  `assume_specification=0`, `cfg_gate=0`. The global `CHEATING_DETECTED` status reflects
  out-of-scope modules (`frame.rs`, `manager.rs`, `mod.rs`), all separately tracked in
  `tcb-allowed.md`.

## Result: PASS
