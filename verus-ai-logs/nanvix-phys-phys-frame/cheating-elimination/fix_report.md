# Cheating Elimination Report: phys-frame

## Scope

Module `mm::phys`. Verification-order target: the `frame.rs` allocator
functions (`Inner::{alloc,alloc_contiguous,free,share,refcount,book,is_covered,
alloc_range}` and their singleton wrappers + `instance`). `init` is
skip/excluded. The cheating detector compiles the whole `mm::phys` module
(`make verify-kernel MODULE=mm::phys`), so it also counts cheating in sibling
modules (`manager`, `kframe`, `mod`, `upool`).

`verus-ai-logs/tcb-allowed.md` exists → its listed functions may keep
`external_body`.

## Cheating Counts (module-wide, before → after)

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 4 | 4 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 18 (all TCB-allowed) | 18 (all TCB-allowed) | 0 |
| assume_specification | per tcb-allowed | unchanged | 0 |
| cfg-gated exec | legitimate only | legitimate only | 0 |

### phys-frame target file (frame.rs / frame.proof.rs / frame.spec.rs) — CLEAN

| Item | Count |
|------|-------|
| admit() | **0** |
| assume() | **0** |
| external_body on proof fns | **0** |
| external_body on exec fns | 10 — every one on `tcb-allowed.md` |
| trusted | 0 |
| exec_allows_no_decreases_clause | 0 |
| multi-line limitation_assume | 0 |

The reviewer's remediation items map as follows:
- `admit()`/`assume()` → the 4 admits are in **`manager.proof.rs`**, not in the
  phys-frame target file. See below.
- `trusted`/`external_body` on **proof fns** → none exist anywhere in `mm::phys`.
- multi-line `limitation_assume` (R20c) → none exist.
- `exec_allows_no_decreases_clause` (R20p) → none exist.

## Items Eliminated

None were eliminable. Investigation findings:

1. **frame.rs is already clean** (0 admit/assume; only TCB-allowed exec
   `external_body`). `git diff HEAD` on all three frame files is empty.
2. **The 4 flagged `admit()`s are pre-existing phys-manager obligations**, in
   `src/kernel/src/mm/phys/manager.proof.rs:{16,35,55,216}`. `git diff
   50e4de7c8 HEAD` (phys-frame START → now) shows phys-frame work touched only
   log files — these admits predate the phase and are out of its target scope.
3. **They are genuinely unprovable in-module** (empirically confirmed). Removing
   them yields `postcondition not satisfied` at manager.proof.rs lines
   14/30/31/47/48/211 — the transition postconditions
   (`m@ == phys_view().frames`, `pre.free_frames.contains(addr)`,
   `post == pre.alloc_one(addr)`, `post == pre.book_all(...)`, `m@ == pre`) are
   not derivable from the lemmas' preconditions.

### Root cause (per-admit detail in verification_todo.md)

`phys_view()` is a parameter-free `uninterp spec fn` with no pre/post temporal
index. `frame::alloc()` mutates the global `frame::INSTANCE` singleton without
borrowing the manager `self`, so Verus cannot see `self@` change. The §8 global
ghost-token that would make this transition expressible
(`view_design.md` §8) has not been realized. The four admits stand in for:
(a) the singleton attachment `self@ == phys_view().frames`
(`lemma_manager_attached`), (b)/(c) the kernel alloc / contiguous-alloc
transitions (`lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`), and
(d) the `Vec::clear()`/`Drop`-based pool restoration
(`lemma_user_bulk_err_restored`) — `Drop` side-effects are not modeled in exec.

### Why no legitimate fix is available in this phase

- They are **external-bottom trust assumptions**. Per **spec-design** /
  **verus-constraints**, `assume_specification`/`axiom` may be authored only
  from the human-approved list; `tcb-allowed.md` does not list them. Authoring
  axioms here would be unapproved cheating.
- Realizing the §8 token requires threading a `Tracked` token **parameter**
  through `frame::alloc`/`Upool::alloc` or adding tracked fields to the singleton
  exec structs — both forbidden exec-signature/struct changes — and is the
  phys-manager proving phase's responsibility.
- Strengthening the lemmas' `requires` only relocates the same undischargeable
  goal to the call sites (`manager.rs:229/258/299/391/510`), which equally
  cannot prove it.

No exec code was modified; no axioms were authored.

## Verification TODOs

See `verus-ai-logs/nanvix-phys-phys-frame/verification_todo.md` — the 4
`manager.proof.rs` admits, each with its exact Verus error and the §8
ghost-token / unmodeled-Drop blocker, plus the two possible resolutions
(human approval to record them as external-bottom assumptions, or §8 token
realization in the phys-manager proving phase).

## AST Consistency

- frame.rs / frame.proof.rs / frame.spec.rs / manager.proof.rs are
  **byte-identical to HEAD** (`git diff HEAD` empty). This phase introduced
  **zero** exec changes.
- Pre-existing `// VERUS BUG FIX:` deviations in 6 `Inner::*` methods are
  documented panic auto-fixes (`bugs.md`) from a prior phase, part of the
  baseline.
- Zero mismatches introduced by this task: **YES**.

## Verification

`make verify-kernel MODULE=mm::phys`: **58 verified, 0 errors** (exit 0). The
module verifies; the cheating gate reports the 4 pre-existing manager admits.

## Result: BLOCKER

The phys-frame target file itself carries **zero** eliminable cheating. The 4
`admit()`s that trip the module-wide gate are pre-existing **phys-manager**
external-bottom obligations (§8 global ghost-token attachment + unmodeled
`Drop` restoration), out of the phys-frame target scope, empirically unprovable
in-module, and eliminable only via human-approved external-bottom axioms or the
phys-manager §8-token proving work — neither within this phase's authority.
Honest hand-off recorded in `verification_todo.md`.
