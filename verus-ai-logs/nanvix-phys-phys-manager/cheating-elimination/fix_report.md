# Cheating Elimination Report: phys-manager

Scope (per task): `src/kernel/src/mm/phys/manager.rs` and its included
`manager.spec.rs` / `manager.proof.rs`. Target functions: `init`,
`alloc_user_frame`, `check_user_watermark`, `alloc_many_user_frames`,
`alloc_many_kernel_frames`, `alloc_kernel_frame`.

## Cheating Counts (before → after)

Module-scoped (`manager.rs` + `manager.spec.rs` + `manager.proof.rs`):

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 6 | 6 | 0 (all in `tcb-allowed.md`) |
| assume_specification | 3 | 3 | 0 (irreducible std-lib specs) |
| cfg-gated exec | 3 | 0 | 3 |

Global `mm::phys` cfg_gate dropped 12 → 9 as a result (the 3 eliminated ones
were the manager's).

## Items Eliminated

- **3 cfg-gated exec (loop invariants).** `manager.rs` lines 235, 473, 492 wrapped
  loop invariants as `#[cfg_attr(verus_keep_ghost, verus_spec(invariant …))]`.
  Per the **verus-constraints** skill (cfg_attr section): wrapping `#[verus_spec(…)]`
  in `cfg_attr(verus_keep_ghost, …)` is redundant and wrong — the macro is already
  ghost and self-erases under a normal `cargo build`. The kernel crate enables
  `#![feature(proc_macro_hygiene)]` **unconditionally** (`kmain.rs:20`), so
  statement-position `#[verus_spec(invariant …)]` is legal in both builds (unlike
  `bitmap`/`slab`, which gate the feature behind `verus_keep_ghost` and therefore
  must keep the `cfg_attr`). Rewrote all three as direct `#[verus_spec(invariant …)]`.
  - Verus: `make verify-kernel MODULE=mm::phys` → 82 verified, 0 errors.
  - Exec: non-verus `cargo check` of the kernel crate compiles cleanly.
  - AST: `ast_consistency.py … count` → consistent (8 fns, 1 struct match) —
    exec code byte-identical after ghost stripping.

## Items Retained (allowed / irreducible)

- **6 `external_body`** — every one is enumerated in
  `verus-ai-logs/tcb-allowed.md`: `init`, `kernel_watermark` (manager.rs) and the
  four §8 ghost-token attachment lemmas (manager.proof.rs). Allowed per the task's
  tcb exception; none is on an unlisted function.
- **3 `assume_specification`** (`Result::and_then`, `Result::inspect_err`,
  `Vec::capacity`) — std/`alloc` methods with no vstd specification (escalation
  ladder: searched vstd — only `Option::and_then` exists, not these). They are the
  standard Verus external-top mechanism for std functions, cannot be verified from
  the kernel crate, and are not counted by the cheating scan (`assume=0`).

## Verification TODOs (`verus-ai-logs/nanvix-phys-phys-manager/verification_todo.md`)

- No proof gaps (zero admit/assume). Only standing, documented trust boundaries
  remain: the 6 tcb-allowed `external_body` items (removed when the frame
  free-function / singleton-bringup layers are verified) and the 3 std-library
  `assume_specification`s (permanent external-top boundaries).

## AST Consistency

- Zero mismatches confirmed: YES (`ast_consistency.py --base-ref
  verus-ai-prove-bottom-up manager.rs count` → "✅ Consistent: 8 functions,
  1 structs match"). The change touched only ghost annotations; exec semantics,
  time complexity, and space complexity are unchanged.

## Verification

- `make verify-kernel MODULE=mm::phys` → 82 verified, 0 errors.
- `make verify` (full crate) → exit 0, 0 verification errors (no regressions).
- Non-verus kernel `cargo check` → compiles cleanly.

## Result: PASS

All cheating eliminable within scope was eliminated (3 cfg-gated exec → 0). The
manager module has zero admit/assume/trusted/no_decreases/cfg_gate. The remaining
6 `external_body` are all tcb-allowed and the 3 `assume_specification`s are
irreducible std-library boundaries not flagged by the gate.
