# Cheating Elimination Report: arch-x86-pde

## Scope

Module: `src/libs/arch/src/x86/mem/paging/pde.rs` (+ `pde.spec.rs`, `pde.proof.rs`).
In-scope functions: `PageDirectoryEntryFlags::new`, `PageDirectoryEntry::new`,
`PageDirectoryEntryFlags::is_present`, `PageDirectoryEntry::is_present`,
`PageDirectoryEntry::frame_address`.

## Cheating Counts (before → after)

Counts are for the **arch-x86-pde module** (the task scope: `pde.rs`/`pde.spec.rs`/`pde.proof.rs`).

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 2 | 0 | 2 |

Crate-wide `make verify-arch` cheating gate: cfg_gate `4 → 2`, external_body `3 → 3`,
assume `0 → 0`, admit `0 → 0`. The remaining items are **all outside the arch-x86-pde
scope** (see "Remaining (out of scope)" below).

Verification result: `make verify-arch` → **48 verified, 0 errors**.
Full-crate `make verify` → arch 48/0, kernel 93/0 (kernel cheating counts unchanged from
baseline — no regression).

## Items Eliminated

Both eliminated items were `cfg-gated exec` markers of the form
`#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]`, one on each `new`:

1. **`PageDirectoryEntryFlags::new`** (was `pde.rs:83`) — the cfg-gated
   `verus_impl_method_marker` allow attribute was a hand-written substitute for the marker
   that `#[verus_verify]` normally auto-emits onto an impl method. It is required because
   the associated function `new` (no `self` receiver) otherwise fails to translate
   (`#[verus_spec]` mis-handles it as a free function: `error[E0425]: cannot find function
   'new' in this scope`).
   **Eliminated** by moving `new` into its own `#[verus_verify] impl PageDirectoryEntryFlags`
   block. `#[verus_verify]` on the impl block auto-generates the same marker at macro
   expansion time (not present in source text, so not a cfg-gate), and — because the block
   contains only the spec'd `new` — it does not drag the unspecified sibling methods
   (`from_raw_value`, `into_raw_value`, `is_user`, …) into verification.

2. **`PageDirectoryEntry::new`** (was `pde.rs:307`) — same root cause and same fix: `new`
   moved into a dedicated `#[verus_verify] impl PageDirectoryEntry` block. The `SIZE`
   const and the unspecified methods stay in plain (external) impl blocks so they are not
   verified.

### Why not the simpler alternatives (escalation ladder)

- Deleting the marker only → `error[E0425]: cannot find function 'new'` (reproduced).
- `#[verus_verify]` directly on the `new` method → marker is **not** added (the macro only
  injects it when applied to an impl *block*); same E0425 (reproduced).
- `#[verus_verify]` on the *whole* original impl block → 16 errors, because the unspecified
  methods (`from_raw_value` → `PresentFlag::from_raw_value`, etc.) get pulled into
  verification without specs (reproduced). This is exactly why the original author reached
  for the cfg-gated marker.
- Splitting the impl block (the accepted fix) gives the marker via `#[verus_verify]` **and**
  keeps the unspecified methods external — zero cfg-gates, zero new obligations.

Root cause traced to the verus-ai macro: `builtin_macros/src/attr_rewrite.rs`
`VerusVerifyVisitor::visit_impl_item_fn_mut` appends `allow(unused, verus_impl_method_marker)`
to every `#[verus_spec]` method of a `#[verus_verify]` impl block; `rewrite_verus_spec_*`
reads that marker to set `is_impl_fn` and strips it before rustc sees it.

## Remaining (out of scope) — not arch-x86-pde, not eliminable here

These trip the crate-wide gate but are **not** in the pde module and the hard rules forbid
touching them. All are TCB-listed or belong to other modules:

- `x86/mem/paging/mod.rs:80 invlpg` — `external_body` (TCB-allowed: inline-asm boundary).
- `x86/mem/paging/table.rs:209 read` / `:246 write` — `external_body` (TCB-allowed:
  int-to-ptr volatile page-table memory).
- `x86/mem/paging/table.proof.rs:16 lemma_entry_roundtrip` — `assume` (table module).
- `x86/mem/paging/pte.rs:85` / `:307` — the same `verus_impl_method_marker` cfg-gates
  (pte module, separate task arch-x86-pte).

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-x86-pde/verification_todo.md)

None. There are zero remaining proof gaps in the arch-x86-pde scope (no `admit`, no `assume`,
no proof holes). All five in-scope functions verify in-body.

## AST Consistency

- Tool: `scripts/ast_consistency.py count`.
- vs pre-task START commit `3e590bcac` (marker pattern): **✅ Consistent — 23 functions,
  2 structs match.**
- vs original parent `3e590bcac~1` (full-block pattern): **✅ Consistent — 23 functions,
  2 structs match.**
- Zero mismatches confirmed: **YES**. Only verification-only attributes and impl-block
  boundaries changed; every exec function body and signature is byte-identical, preserving
  semantics, time complexity, and space complexity (a Rust impl-block split is exec-neutral).

## Result: PASS
