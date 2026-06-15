# Cheating Elimination Report: phys-frame

Module: `mm::phys` — file `src/kernel/src/mm/phys/frame.rs`
(+ ghost `frame.spec.rs`, `frame.proof.rs`).
Base branch: `verus-ai/phys-kframe`. Working branch: `verus-ai/phys-frame`.

## Cheating Counts (before → after)

Counts are for the proof-target file `frame.rs` (and its ghost includes).
`external_body` rows count functions that carry `#[verus_verify(external_body)]`;
all 11 are listed in `verus-ai-logs/tcb-allowed.md` and are therefore **allowed**
trust boundaries, not blockers.

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 11     | 11    | 0 (all allow-listed) |
| assume_specification | 0      | 0     | 0          |
| cfg-gated exec       | 0      | 0     | 0          |

Verification result: **module `mm::phys` = 31 verified, 0 errors**; full kernel
crate = **32 verified, 0 errors**; full multi-crate `make verify` = all crates
exit 0.

## Items Eliminated

No cheating item required elimination in this pass: the file was already free of
`admit()`, `assume()`, `assume_specification`, and non-allow-listed
`external_body`, and the verifier reports **0 errors** (no proof gaps).

For the record, the obstacle a prior pass had recorded (6 mutating shims left as
`proof! { admit(); }`) is **already resolved** on this branch — not by weakening
specs but by threading a `Tracked<&mut PhysAuth>` carrier through each mutating
shim. Each shim now binds `let r = instance(); let res = r.<op>(...); proof! {
auth.v.frames = (*r)@; } res`, where the `proof!` block re-pins the ghost view to
the post-mutation `Inner` view so the strong `old(auth)@ → final(auth)@`
post-state contracts (`spec_alloc_one` / `spec_alloc_set` / `spec_share`) verify.
`free_count` is discharged by `lemma_free_count` in `frame.proof.rs`. These were
already committed; this pass confirms 0 residual cheating.

### `external_body` retained (all allow-listed in `tcb-allowed.md`)

- `Inner::alloc`, `Inner::alloc_contiguous`, `Inner::free`, `Inner::share`,
  `Inner::refcount`, `Inner::book`, `Inner::is_covered`, `Inner::alloc_range` —
  bodies need `core::fmt::Arguments` (`error!`/`debug_assert_eq!`) and the `arch`
  newtypes `FrameNumber`/`FrameAddress` (no `external_type_specification`), both
  unsupported by the Verus front end. Their `old(self)@ → final(self)@` contracts
  are the trust boundary; the body-verified `frame::*` shims are checked against
  them.
- `instance` — `static mut INSTANCE` singleton bridge axiom (unsupported
  `static mut` paths).
- `free` (Drop path) — `UserFrame`/`KernelFrame::drop` are `opens_invariants
  none` + `no_unwind`, so no `PhysAuth` carrier can be threaded.
- `init` — skip/exclude target (materializes `&'static mut [u8]`, writes
  `static mut INSTANCE`).

## Verification TODOs (`verus-ai-logs/nanvix-phys-phys-frame/verification_todo.md`)

None. There are **no genuinely-stuck proofs**. All in-scope shims are
body-verified; the remaining `external_body` items are governed trust boundaries
listed in `tcb-allowed.md`, not proof gaps.

## AST Consistency

- Zero mismatches confirmed: **YES**.
- Diff of `frame.rs` vs `verus-ai/phys-kframe` touches only the singleton/shim
  region (hunks at lines 633+); every `Inner` struct field and every `Inner::*`
  method **body** is byte-identical to the base branch.
- The only exec-body deviations are in the free-function shims and `free_count`:
  zero-cost `let` bindings plus ghost `proof!` blocks (`auth.v.frames = (*r)@;`,
  `lemma_free_count(inner)`). The `proof!` blocks are erased at compile time and
  the `let` bindings are zero-cost, so **semantics, time complexity, and space
  complexity are preserved**. The bindings are required by Verus: a `proof!`
  block must name `r` to reference `(*r)@`, and `res` to return after the ghost
  update — this is the mechanism that carries the strong `PhysAuth` post-state
  contract. `frame.proof.rs` adds only the ghost `lemma_free_count`;
  `frame.spec.rs` is unchanged.

## Result: PASS
