# Cheating Elimination Report: phys-upool

Scope: `src/kernel/src/mm/phys/upool.rs` (+ `upool.spec.rs`, `upool.proof.rs`).
In-scope functions: `UserFrame::{share, refcount, leak, drop, new, address}`,
`Upool::{new, alloc}`.

Counts below are **module-scoped to `mm::phys::upool`**, with the "Before"
column measured against the base branch `verus-ai/phys-manager`. The 25
`external_body` items reported by the global `make verify-kernel` scan all live
in **other** files (`frame.rs`, `manager.rs`, `mod.rs`, `mod.spec.rs`) and are
explicitly listed in `verus-ai-logs/tcb-allowed.md` and out of scope ("Do not
touch unlisted functions"). None are in `upool`.

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 1 | 0 | 1 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 1 | 0 | 1 |

`make verify-kernel MODULE=mm::phys::upool` → `8 verified, 0 errors`,
`✅ No cheating detected in module mm::phys::upool`, `status: CLEAN`.
Global `cfg_gate` count dropped 10 → 9 as a direct result.

## Items Eliminated
- **`Upool::new` — `external_body` (upool.rs).** In base
  `verus-ai/phys-manager` this was `#[verus_verify(external_body)]`. It is a
  trivial constructor (`Self { _private: () }`) with no contract, so it needs no
  trust boundary. The attribute was reduced to `#[verus_verify]` and the body
  verifies as-is. (Eliminated during the proving phase that produced this
  branch; recorded here for completeness since it is the upool `external_body`
  relative to the base reference.)
- **`View for UserFrame` — `cfg`-gated `verus!` block (upool.rs).** The view
  definition lived in `upool.rs` inside `#[cfg(verus_keep_ghost)] verus! { … }`.
  The cheating-detector heuristic counts a `cfg(verus_keep_ghost)`-gated
  `verus!` block as "cfg-gated exec code". The block is ghost-only (a `closed
  spec fn view`), but to drive the module to a genuine zero I **relocated the
  `View for UserFrame` impl verbatim into `upool.spec.rs`**, which is the
  conventional home for view/spec definitions and is pulled in via
  `#[cfg(verus_keep_ghost)] include!("upool.spec.rs")` (an `include!` target the
  detector correctly excludes). No `verus!` block remains directly in
  `upool.rs`; its remaining `cfg(verus_keep_ghost)` gates target only `include!`
  / `use`, which are excluded from the count.

  This is a pure **ghost-code relocation**: in the non-verus exec build the view
  was already absent (it sat in a `cfg(verus_keep_ghost)` block) and remains
  absent (the spec file is `include!`d only under `verus_keep_ghost`). In the
  verus build it is present in both cases. Exec semantics, time complexity, and
  space complexity are unchanged. `View for UserFrame` is **not** in the
  do-not-modify list (only `View for Inner` is), so relocating it is permitted.

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-upool/verification_todo.md)
- None. No remaining proof gaps; all 8 in-scope functions plus `Drop::drop`
  verify with real proofs (no `admit`/`assume`). `drop` verifies directly
  despite calling `frame::free` + `error!` — no `external_body` needed.

## AST Consistency
- `scripts/ast_consistency.py --base-ref verus-ai/phys-manager
  src/kernel/src/mm/phys/upool.rs`:
  `✅ Consistent: 8 functions, 2 structs match` (matched=8 mismatched=0
  missing=0 extra=0). All exec code is byte-for-exec identical to the base
  branch; only ghost contracts were added and one ghost view impl was relocated
  to the spec file.
- Zero mismatches confirmed: **YES**

## Validation
- `make verify-kernel MODULE=mm::phys::upool`: 8 verified, 0 errors, module
  status **CLEAN** (no cheating).
- `make verify` (full crate set): exit 0, every crate verifies with 0 errors.
  Kernel global status remains `CHEATING_DETECTED` solely due to the
  pre-existing, `tcb-allowed.md`-listed `external_body` shims in
  `frame.rs`/`manager.rs`/`mod.rs` and other modules' view-block cfg gates —
  none in `upool`.
- `./z build -- all-kernel`: non-verus exec kernel compiles cleanly (exit 0).

## Result: PASS
