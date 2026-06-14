# Cheating Elimination Report: phys-mod

Scope: `src/kernel/src/mm/phys/mod.rs` (+ `mod.spec.rs`, `mod.proof.rs`).
In-scope functions: `init`, `book_physical_memory_regions`, `book_mmio_regions`.
Out-of-scope (separate tasks/modules): `frame.rs`, `manager.rs`, `upool.rs`,
`kframe.rs`.

## Cheating Counts (before → after) — in-scope phys-mod only
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 2 (both TCB-allowed) | 2 (both TCB-allowed) | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Supporting trusted declaration (not a function, not a proof gap):
`mod.spec.rs:73 ExLinkedList` — `#[verifier::external_type_specification]`
(external-bottom trusted type declaration; see below).

Crate-global cheating reported by the gate (`external_body=17 cfg_gate=5`) is
dominated by out-of-scope modules (`frame.rs` ×12, `manager.rs` ×1, `upool.rs`
×1) which are owned by their own verification tasks and are not touched here.

## Items Eliminated
None required elimination in scope. The remaining in-scope items are all
explicitly permitted:

- `book_physical_memory_regions` (`mod.rs:82`) — `external_body`, **TCB-allowed**
  (`verus-ai-logs/tcb-allowed.md`). Iterates a std
  `alloc::collections::LinkedList` via `for region in list.iter()`.
- `book_mmio_regions` (`mod.rs:117`) — `external_body`, **TCB-allowed**. Same std
  `LinkedList` iteration limitation.
- `ExLinkedList` (`mod.spec.rs:73`) — `#[verifier::external_type_specification]`
  with `#[verifier::external_body]`. This is the *required* trusted type
  declaration that lets verified code (`init`) name the foreign `LinkedList`
  type. It is an external-bottom trusted spec, has no provable body, and is the
  supporting declaration for the two TCB-allowed helpers above.

### Escalation-ladder evidence (verus-constraints)
The two `external_body` helpers cannot be body-verified — confirmed, not assumed:
1. **Searched vstd** (`~/toolchain/verus/vstd/std_specs/`): iterator ghost-specs
   exist only for `slice`, `vec`, `vecdeque`, `range`, `iter` — **no
   `LinkedList`** (`grep -rin LinkedList` over vstd returns nothing).
2. **Orphan rule**: supplying `View` / `ForLoopGhostIterator` for the foreign
   `alloc::collections::linked_list::Iter` from the `kernel` crate is rejected
   with `error[E0117]` (trait and type both foreign).
3. **Equivalent rewrite blocked**: `vstd` is exact-pinned and cannot be extended
   in-tree; switching the region containers from `LinkedList` to `Vec`/slice
   would change the public boot interface (out of scope, and an exec/ABI change).

The *abstract* booking effect is fully specified over `PhysMemView`
(`spec_book_frame`, `spec_book_frames`, `region_frames`) and **proven with no
`admit`** by `lemma_book_region_reserves_region_frames`,
`lemma_book_mmio_skip_untracked`, `lemma_book_mmio_books_tracked`,
`lemma_spec_book_frames_preserves_inv`, and
`lemma_init_establishes_and_reserves` in `mod.proof.rs`. Full analysis:
`verus-ai-logs/nanvix-phys-phys-mod/verus-unsupported.md`.

`init` is genuinely verified (real postcondition: `phys_view().inv()`; on `Ok`,
`initialized` and `allocated_frames` disjoint from `free_frames`) — no cheating.

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-mod/verification_todo.md)
None. No proof gaps remain; no `verification_todo.md` is required. The only
trusted boundaries are the two TCB-allowed `external_body` helpers (genuine Verus
front-end limitation for std `LinkedList` iteration, re-evaluation trigger
recorded in `verus-unsupported.md`) and their supporting `ExLinkedList` type
declaration.

## AST Consistency
- Zero mismatches confirmed: YES.
- Diff vs `verus-ai/bump-allocator` for the three phys-mod files is purely
  additive (461 insertions, 0 deletions): new `mod.proof.rs`, plus
  ghost-only/spec-only additions to `mod.rs` (`#[verus_spec]` / `#[verus_verify]`
  attributes, `#[cfg(verus_keep_ghost)]` includes) and `mod.spec.rs`.
- Executable bodies of `init`, `book_physical_memory_regions`,
  `book_mmio_regions` are **identical** to base — no exec-logic, time-complexity,
  or space-complexity changes; all proof scaffolding compiles out of non-Verus
  builds. No cfg-gated exec divergence introduced.

## Verification Results
- `make verify-kernel MODULE=mm::phys` (fresh): **9 verified, 0 errors** (exit 0).
- `make verify-kernel` (full kernel crate): **exit 0**, `assume=0 admit=0`.
  (`external_body=17 cfg_gate=5` are crate-global and out-of-scope.)

## Result: PASS
In-scope phys-mod cheating: `admit=0`, `assume=0`, `assume_specification=0`,
cfg-gated-exec=0. The only remaining `external_body` are the two
explicitly TCB-allowed `LinkedList` helpers plus their required `ExLinkedList`
trusted type declaration. No proof gaps. AST consistency clean.
