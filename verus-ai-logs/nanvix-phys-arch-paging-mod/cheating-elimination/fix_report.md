# Cheating Elimination Report: arch-paging-mod

## Scope

In-scope target function: `invlpg` (`mod.rs`). The cheating gate for this phase
scans the whole `paging` module subtree that `mod.rs` includes (`table`, `pde`,
`pte`, `flags`, `frame`), so the report below covers every cheating construct the
gate counts under this phase.

## Cheating Counts (before → after) — `make verify-arch`
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 1 | 0 | 1 |
| assume() | 0 | 0 | 0 |
| external_body | 3 (all allowlisted) | 3 (all allowlisted) | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

`cfg_gate=4` reported by the tool = the four `#[cfg(verus_keep_ghost)] include!("…")`
spec/proof guards in `mod.rs` and `table.rs`. These gate **spec/proof inclusion**, the
project-standard pattern — **not** exec code — so they are not a cfg-gated-exec
deviation (per the ast-consistency skill).

## Items Eliminated

### `admit()` — `table.proof.rs::lemma_entry_roundtrip` (ELIMINATED)

- **Was:** a `pub broadcast proof fn lemma_entry_roundtrip<E>(e: E)` asserting the
  codec round-trip law `spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e)` with
  body `admit()` — an undischarged specification-phase placeholder.
- **Escalation ladder (verus-constraints):**
  1. *Search vstd* — no applicable lemma; the proposition is domain-specific over the
     crate's own `uninterp` codec functions.
  2. *Isolated analysis* — `spec_entry_raw` / `spec_entry_from_raw` are
     `uninterp spec fn` and the lemma is generic over `E` with **no** `TableEntry`
     bound, so the body has zero usable facts: the statement is a pure axiom and is
     **unprovable as written**. A real proof needs per-implementor bit-level codec
     reasoning (`pde.rs` / `pte.rs` / `flags.rs`, none of which carry contracts yet) —
     the not-yet-run `table` proving phase.
  3. *Equivalent rewrite* — confirmed the lemma is **dead code**: it is never
     `broadcast use`d, in no broadcast group, and has no caller anywhere in the repo
     (`grep -rn` across `src/`). No proof depends on it.
- **How eliminated:** removed the dead placeholder (replaced by a documentation
  comment explaining the deferral). It was **not** swapped for `assume` /
  `assume_specification` / `external_body` (each of which would also be cheating).
  Result: `admit=0`, `assume=0`. Proper proof recorded as a deferred item (see TODOs).

### `external_body` ×3 — all TCB-allowlisted (RETAINED, permitted)

The task permits `external_body` for functions listed in `verus-ai-logs/tcb-allowed.md`.
All three are listed there:
- `mod.rs::invlpg` — single `core::arch::asm!` issuing `invlpg`; inline asm is
  unsupported by Verus (external-bottom hardware TLB boundary). Empty faithful
  contract; matches the inherited upstream `assume_specification`.
- `table.rs::Table::<E>::read` — `usize`→`*const` materialization + volatile load of
  externally-owned page-table memory; full `#[verus_spec]` pinned to the global
  page-table ghost.
- `table.rs::Table::<E>::write` — `usize`→`*mut` materialization + volatile store;
  sound `requires index@ < PAGE_TABLE_LENGTH`.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-paging-mod/verification_todo.md)

- `TableEntry` round-trip law: the genuine proof is deferred to the `table` proving
  phase, where the trait gains a `proof fn lemma_roundtrip` obligation discharged by
  each implementor once `from_raw_value`/`into_raw_value` and the flag/frame codecs
  receive real `#[verus_spec]` contracts. No `admit`/`assume` remains in the tree, so
  this TODO does not trip the cheating gate.

## AST Consistency

- Zero mismatches confirmed: **YES**.
  - The only source change is `table.proof.rs` — a **proof-only** file (`#[cfg(verus_keep_ghost)]`-included). A dead `proof fn` was deleted; no exec code, no
    function signature, no `mod.rs`/`invlpg` line changed.
  - `git diff <pre-task> -- 'src/*.rs'` touches only `table.proof.rs`.
  - No exec semantics, time complexity, or space complexity affected (the removed item
    was ghost-only and unused).

## Verification Results

- `make verify-arch`: **47 verified, 0 errors** — `assume=0 admit=0`,
  `external_body=3` (all allowlisted).
- `make verify` (full): arch **47 verified, 0 errors**; kernel **76 verified,
  0 errors**. No regressions introduced. (Kernel-side `admit`/`external_body` counts
  are pre-existing items owned by other phases, unchanged by this edit.)

## Result: PASS

All eliminable cheating in scope is gone: `admit=0`, `assume=0`,
`assume_specification=0`, no cfg-gated exec. The remaining `external_body=3` are
exactly the three functions on the TCB allowlist (`invlpg`, `table::read`,
`table::write`), which the task explicitly permits.
