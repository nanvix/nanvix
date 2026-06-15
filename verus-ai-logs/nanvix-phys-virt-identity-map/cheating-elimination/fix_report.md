# Cheating Elimination Report: virt-identity-map

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 3 | 3 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Notes on the counts:
- `external_body = 3` are the three in-scope functions `ensure_pt`, `ensure_pte`,
  `identity_map_page` — **all explicitly listed in `verus-ai-logs/tcb-allowed.md`**
  ("Allowed `external_body` — `mm::virt::identity_map` (proof target)", lines
  182–219). The task's exception clause permits tcb-listed functions to keep
  `external_body`; none are blockers.
- The 4 `external_type_specification` registrations in `identity_map.spec.rs`
  (`ExTableIndex`, `ExPageDirectoryEntry`, `ExPageTableEntry`, `ExTable`) are
  classified by the verification harness as `external_type_spec`, **not** counted as
  `external_body`. They are the mandatory opaque-external-type idiom (identical to
  the tcb-sanctioned `ExLinkedList` / `ExFrameNumber`) required to let the foreign
  `arch` paging types appear in the in-scope signatures. `external_type_specification`
  cannot exist without `external_body`; it is a type registration, not a function
  trust gap.
- The two `#[cfg(verus_keep_ghost)]` gates only `include!` the `.spec.rs` / `.proof.rs`
  files. No exec function diverges between Verus and non-Verus builds → 0 cfg-gated
  exec.

## Items Eliminated
None were eliminable. The module entered this phase with **zero** `admit()`, `assume()`,
or `assume_specification`, and the only `external_body` present are the three
tcb-sanctioned proof-target shims plus the required type-spec idiom.

Investigation per the verus-constraints escalation ladder:
- The three shims' contracts are stated over the **uninterpreted** `identity_map_view()`
  accessor (`uninterp spec fn`, no definition). Body verification is structurally
  impossible: the verifier cannot derive `identity_map_view().inv()` /
  `.maps(phys_addr)` from a body that reads/writes raw page-table memory through
  `arch::Table` (volatile pointers, no `PointsTo` model), loads the module-global
  `static KERNEL_PD_PADDR`, draws from the interior-mutable `static PAGE_TABLE_ALLOCATOR`,
  builds entries via unspecced `arch` newtype/enum constructors
  (`PageDirectoryEntry::new`, `FrameNumber::from_raw_value`, the `*Flag` enums), and
  issues the `paging::invlpg` inline-asm. This is the same trusted-boundary pattern as
  `mm::phys`'s `phys_view()` / `instance()` bridge.
- Eliminating them would require modeling all of `arch` paging plus a page-table memory
  model — out of scope ("Do not touch unlisted functions") and explicitly below this
  module's verification boundary per `view_design.md`.
- The four abstract laws in `identity_map.proof.rs` carry the transition vocabulary
  (idempotence, map-on-success, monotone growth, invariant preservation) and are fully
  proven (no `admit()` bodies); they verify cleanly.

## Verification TODOs (verus-ai-logs/nanvix-phys-virt-identity-map/verification_todo.md)
- No actionable, in-scope proof gaps. The retained trust surface is the three
  tcb-allowed `external_body` shims, removable only when the `arch` paging types and a
  page-table memory model are verified (out of scope). Recorded for honest hand-off.

## AST Consistency
- Zero exec mismatches confirmed: YES. No edits were made in this session. Versus the
  base `verus-ai/arch-frame-number`, the only **removed** line in `identity_map.rs` is
  `use ::vstd::prelude::*;` → `use vstd::prelude::*;` (a trivial ghost-only import-path
  normalization; `vstd` is present only under `verus_keep_ghost`). Every other diff is
  purely additive: the ghost-gated `include!` of `.spec.rs`/`.proof.rs` and the
  `#[verus_spec]` / `#[verus_verify]` contract attributes. No exec function-body
  statement was added, removed, or reordered — semantics, time complexity, and space
  complexity of `ensure_pt`, `ensure_pte`, and `identity_map_page` are identical to the
  base. The `.spec.rs` / `.proof.rs` files are new ghost-only artifacts.

## Verification
- `make verify-kernel MODULE=mm::virt::identity_map` → exit 0; module cheating:
  assume=0, external_body=3 (all tcb-sanctioned), admit=0.
- `make verify-kernel` (full kernel) → exit 0; assume=0, admit=0, no_decreases=0.
- `make verify` (full crate set) → exit 0; no regressions (zero source changes).

## Result: PASS
All cheating present in scope is sanctioned by `verus-ai-logs/tcb-allowed.md` (the three
proof-target `external_body` shims) or is the mandatory `external_type_specification`
idiom. There are zero `admit()`, `assume()`, `assume_specification`, or cfg-gated exec
items, and zero non-sanctioned `external_body`. Verification passes at exit 0 with no
regressions.
