# Cheating Elimination Report: phys-frame

## Scope

Module `mm::phys`, file `src/kernel/src/mm/phys/frame.rs` (+ `frame.spec.rs`,
`frame.proof.rs`). Target functions: the `Inner::*` frame-allocator methods
(`alloc`, `alloc_contiguous`, `alloc_range`, `book`, `free`, `share`,
`refcount`, `is_covered`) and their singleton wrappers (`alloc`,
`alloc_contiguous`, `free_count`, `free`, `book`, `alloc_range`, `share`,
`refcount`, `is_covered`), plus `instance`. `init` is skip/excluded.

`verus-ai-logs/tcb-allowed.md` **exists**, so functions listed there may keep
`external_body`.

## Cheating Counts (before → after) — phys-frame module only

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 10 (all TCB-allowed) | 10 (all TCB-allowed) | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | legitimate only | legitimate only | 0 |

No non-allowed cheating item existed in the frame module at task start, so there
was nothing eliminable. Details below.

### `external_body` inventory (frame.rs) — all on the TCB-allowed list

| Fn (line) | TCB-allowed entry |
|-----------|-------------------|
| `instance` (1408) | "Allowed `external_body`" — int-to-ptr materialization of the `static mut INSTANCE` singleton |
| `init` (1446) | "Skip / exclude from current proof target" + cross-module deps |
| `alloc` (1502) | Cross-module deps: singleton wrapper, post-mutation `phys_view()` deferral |
| `alloc_contiguous` (1532) | Cross-module deps: singleton wrapper |
| `free_count` (1553) | Cross-module deps: reports free-partition size |
| `free` (1571) | Cross-module deps: best-effort release |
| `book` (1613) | Cross-module deps: post-mutation `phys_view()` deferral |
| `alloc_range` (1634) | Cross-module deps: post-mutation `phys_view()` deferral |
| `share` (1654) | Cross-module deps: CoW refcount bump |
| `refcount` (1675) | Cross-module deps: pure read |

The singleton wrappers are deliberately deferred (`external_body`) because their
postconditions reference the *post-mutation* parameter-free global ghost
`phys_view().frames`, which cannot be expressed until the §8 ghost token is
realized in the free-function layer. This deferral is human-approved in
`tcb-allowed.md`. The `Inner::*` methods that hold the real state transitions are
verified **in-body** (no `external_body`).

Note: the singleton wrapper `is_covered` (1593) is **not** `external_body` — it
is verified in-body.

### cfg-gated exec (legitimate, not cheating)

All `#[cfg(not(verus_keep_ghost))]` sites in frame.rs gate Verus-unsupported
sub-expressions only — `error!(...)` logging macros, `debug_assert_eq!(...)`,
and logging-only `let` bindings (`uncovered_addr`, `conflicting_addr`). None
gate-and-duplicate an exec definition. This is the allowed pattern in the
verus-constraints skill ("cfg-gate the unsupported sub-expression, never skip a
whole function").

## Items Eliminated

None required. At task start the frame module already had:
- 0 `admit()`, 0 `assume()`, 0 `assume_specification` (frame.spec.rs declares
  only `Inner::inv`; all former placeholder `assume_specification` /
  `external_type_specification` were already superseded and removed — see the
  documentation comments at frame.spec.rs:20–36).
- 10 `external_body`, every one on the TCB-allowed list.
- All in-scope `Inner::*` proofs discharged in-body.

Verification was confirmed fresh (non-cached): `make verify-kernel MODULE=mm::phys`
reports **58 verified, 0 errors (exit 0)** — i.e. zero proof gaps in the module.
Full-crate `make verify` reports exit 0 (no regressions).

The global cheating counter (`external_body=19 admit=16 cfg_gate=19`) aggregates
the **entire kernel crate**; the residual admit/external_body live in
out-of-scope modules (`manager`, `kframe`, `mod`, `upool`, `hal/...`), which this
task must not touch. For the frame module itself, non-allowed cheating = 0.

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-frame/verification_todo.md)

None. No proof gap, `admit`, or `assume` remains in the frame module, so no
hand-off file was created.

## AST Consistency

- Frame files are **byte-identical** to the task baseline
  (`git diff --stat HEAD -- frame.rs frame.proof.rs frame.spec.rs` is empty).
  This task introduced **zero** exec changes.
- Zero mismatches introduced by this task: **YES**.
- Pre-existing deviations vs upstream `dev`: 6 `Inner::*` methods
  (`is_covered`, `book`, `free`, `share`, `refcount`, `alloc_range`) replace
  `into_frame_number().into_raw_value()` with `into_raw_value() / mem::FRAME_SIZE`.
  These are documented `// VERUS BUG FIX:` auto-fixes for a genuine
  panic-on-valid-input bug (top-of-space aligned address), recorded in
  `verus-ai-logs/nanvix-phys-phys-frame/bugs.md` ("[auto-fixed]
  panic-on-valid-input"). They were committed in a prior phase, are part of the
  baseline, and are outside this cheating-elimination task's scope.

## Result: PASS
