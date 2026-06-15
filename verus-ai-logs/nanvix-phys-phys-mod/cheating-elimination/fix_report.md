# Cheating Elimination Report: phys-mod

Module: `mm::phys` — file `src/kernel/src/mm/phys/mod.rs` (+ `mod.spec.rs`, `mod.proof.rs`).
In-scope functions: `init`, `book_physical_memory_regions`, `book_mmio_regions`.
Command: `make verify-kernel MODULE=mm::phys` → **86 verified, 0 errors** (exit 0).

## Cheating Counts (before → after)

Counts below are for the **phys-mod scope** (`mod.rs` + `mod.spec.rs` + `mod.proof.rs`).

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 3*     | 3*    | 0          |
| assume_specification | 0      | 0     | 0          |
| cfg-gated exec       | 0      | 0     | 0          |

\* All 3 `external_body` are **TCB-allowed** (`verus-ai-logs/tcb-allowed.md`):
`book_physical_memory_regions`, `book_mmio_regions`, and the `ExLinkedList`
`external_type_specification` in `mod.spec.rs`. None is on a non-allowed function.

The in-scope source files are byte-identical to the base branch
(`git diff verus-ai-prove-bottom-up -- mod.rs mod.spec.rs mod.proof.rs` is empty): the
proving phase already left phys-mod fully verified, so cheating-elimination had no
non-allowed item to remove.

> Crate-global counts reported by the cheating gate (`assume=0 external_body=14 admit=3
> cfg_gate=9`) include items **outside phys-mod scope**: `frame.rs`/`manager.rs`/`upool.rs`
> `external_body` (all TCB-allowed), the four `manager.proof.rs` `assume`s (approved
> `L60`–`L63` in `verus-ai-logs/approved-trust-boundaries.json`, hence `assume=0`), and the
> `admit=3` in `mm/virt/identity_map.rs` — a **different module** (`mm::virt`), not part of
> the phys-mod scope and not touchable here (hard rule: do not touch unlisted functions).

## Items Eliminated

None required elimination. Every cheating item inside the phys-mod scope is a pre-approved
TCB boundary:

- `init` — no cheating; machine-verified with a real `#[verus_spec]` contract and a real exec
  body. Postconditions discharged from callee contracts; `mod.proof.rs` needs no lemma body.
- `book_physical_memory_regions` / `book_mmio_regions` — TCB-allowed `external_body`
  (foreign `LinkedList` `for`-loop iteration). Escalation ladder exhausted: `vstd` has **no**
  `LinkedList` model (`grep LinkedList ~/toolchain/verus` → empty); the orphan rule blocks
  implementing vstd's `View`/`ForLoopGhostIterator` for the foreign type; an equivalent
  rewrite would change exec data structures + caller signatures (ast-consistency violation).
  Both keep real `ensures` (booked frames become reserved / covered MMIO frames reserved).
- `ExLinkedList` (`mod.spec.rs`) — TCB-allowed `external_type_specification` (mandatory
  `external_body`); the only sanctioned way to name the unparseable foreign `LinkedList` in
  spec signatures.

## Verification TODOs (`verus-ai-logs/nanvix-phys-phys-mod/verification_todo.md`)

No remaining proof gaps in scope (zero `admit()`/`assume()`). `init` is fully proven. The
file records the two foreign-`LinkedList` `external_body` functions + `ExLinkedList` as
permanent TCB boundaries (not stuck proofs), with the exhausted escalation-ladder evidence.

## AST Consistency

- Zero mismatches confirmed: **YES**. The in-scope exec file `mod.rs` (and `mod.spec.rs`,
  `mod.proof.rs`) are byte-identical to `verus-ai-prove-bottom-up` — no exec-code changes, no
  cfg-gated exec divergence introduced. The only `#[cfg(verus_keep_ghost)]` gates in `mod.rs`
  (lines 36/40/42) are the standard verus harness (`use vstd`, `include!` spec/proof) and gate
  no exec behavior. Semantics, time and space complexity trivially preserved (no edits).

## Result: PASS

`make verify-kernel MODULE=mm::phys` → 86 verified, 0 errors. Full-crate `make verify-kernel`
and `make verify` → exit 0 (no regressions). All phys-mod-scope cheating is TCB-allowed; zero
non-allowed `admit`/`assume`/`external_body`/`assume_specification`/cfg-gated-exec remain in
scope.
