# Cheating Elimination Report: phys-mod

Scope: `src/kernel/src/mm/phys/mod.rs`, `mod.spec.rs`, `mod.proof.rs`.
In-scope functions: `init`, `book_mmio_regions`, `book_physical_memory_regions`.
(The submodules `frame`, `kframe`, `manager`, `upool` reported by `MODULE=mm::phys`
are **out of scope** — they have their own phases — and were not touched.)

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 2 (both TCB-allowed) | 2 (both TCB-allowed) | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

(`external_type_specification` `ExLinkedList` in `mod.spec.rs` is required spec
infrastructure to name `LinkedList` in signatures — not an exec-function cheat.)

## Items Eliminated
- None required. The module was already at its correct terminal state:
  - `init` — fully body-verified (no `external_body`); discharges its postcondition
    directly from the contracts of `frame::init` and `PhysMemoryManager::init` over the
    parameter-free `phys_view()` accessor. No proof gap.
  - `book_physical_memory_regions` (mod.rs:73) — `external_body`, **listed in
    `tcb-allowed.md`**. Allowed to remain.
  - `book_mmio_regions` (mod.rs:103) — `external_body`, **listed in `tcb-allowed.md`**.
    Allowed to remain.
- No `admit()`, `assume()`, `assume_specification`, or cfg-gated exec code exists in any
  of the three module files, so there was nothing in those categories to eliminate.

### Escalation-ladder due diligence on the two TCB-allowed `external_body`
1. **Searched vstd**: `~/toolchain/verus/vstd/std_specs/` provides `vec`, `vecdeque`,
   `btree` — but **no `linked_list`**. `grep -rl LinkedList` over the pinned `vstd`
   returns nothing. No model exists for `alloc::collections::LinkedList`.
2. **Orphan-rule blocker**: implementing vstd's `View` / `ForLoopGhostIterator(New)` for
   the foreign `LinkedList` / `linked_list::Iter` from the kernel crate is a hard Rust
   error (E0117). Only `vstd` may provide it. Registering the type (`ExLinkedList`) lets
   it appear in signatures but supplies no iteration semantics.
3. **Equivalent rewrite**: switching the containers to `Vec`/`VecDeque` would change the
   public `init` signature and the exec data structure (ast-consistency violation:
   different semantics/complexity and a breaking API change), and the postconditions use
   the necessarily-`uninterp` `phys_regions_frame_set` / `mmio_regions_frame_set`, which
   have no concrete model to tie loop bodies to. Not viable.
Conclusion: the blocker is genuine and matches the pre-approved `tcb-allowed.md` entries.
Recorded in `verification_todo.md` as the honest resolution path.

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-mod/verification_todo.md)
- `book_physical_memory_regions` — `for` loop over `LinkedList`; blocked by absent vstd
  `LinkedList` model + orphan rule (E0117). TCB-allowed; contract pins Ok ⇒ all physical
  region frames reserved.
- `book_mmio_regions` — same `LinkedList` blocker. TCB-allowed; contract pins Ok ⇒ all
  *covered* MMIO frames reserved (uncovered skipped).

## AST Consistency
- Zero mismatches confirmed: YES.
  `git diff verus-ai-prove -- mod.rs mod.spec.rs mod.proof.rs` is empty — the exec code
  is byte-identical to the base branch. No cfg gates, no semantic/complexity changes.

## Verification
- `make verify-kernel MODULE=mm::phys` — exit code 0 (verification passes; every specced
  function verifies). The only flagged items in scope are the two TCB-allowed
  `external_body` helpers; in-scope `admit`/`assume` = 0.

## Result: PASS
