# Cheating Elimination Report: arch-x86-pde

## Cheating Counts (before → after)

Scope = `src/libs/arch/src/x86/mem/paging/pde.rs` (+ `pde.spec.rs`, `pde.proof.rs`).
Counts shown are crate-wide (`make verify-arch`); the only in-scope cheating was the
two `cfg-gated exec` items in `pde.rs`.

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 3 | 3 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 2 | 0 | 2 |

Notes:
- The 3 remaining `external_body` are **not** in scope and are all on the
  TCB-allowed list (`verus-ai-logs/tcb-allowed.md`): `x86/mem/paging/mod.rs::invlpg`,
  `x86/mem/paging/table.rs::Table::read`, `x86/mem/paging/table.rs::Table::write`.
  None are in `pde.rs`.
- The 2 `cfg-gated exec` items were both in `pde.rs` and are fully eliminated.

## Items Eliminated

Both eliminated items were the attribute
`#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]` placed on the
two constructors `PageDirectoryEntryFlags::new` (line 84) and
`PageDirectoryEntry::new` (line 309). The cheating-detector counts any
`#[cfg_attr(verus_keep_ghost, …)]` as cfg-gated exec.

Root cause: `verus_impl_method_marker` is the internal flag the `#[verus_spec]` macro
needs to know it is rewriting an **impl method**. For a constructor with no `self`
receiver (`new`), without this flag the macro emits a bare `new(...)` call in the
generated type-constraint (instead of `Self::new(...)`), producing
`error[E0425]: cannot find function 'new' in this scope`. `self`-receiver methods
(`is_present`, `frame_address`) do not need it. The marker was being injected manually
via a `verus_keep_ghost` cfg-gate because, unconditionally, `verus_impl_method_marker`
is an unknown lint in a normal `cargo build`.

Fix (idiomatic, matching the sibling `pte.rs`): instead of manually cfg-gating the
marker, the `#[verus_verify]` attribute on the **impl block** auto-injects
`#[allow(unused, verus_impl_method_marker)]` for every spec'd method
(`builtin_macros::attr_rewrite::VerusVerifyVisitor::visit_impl_item_fn_mut`). So each
`new` was moved into its own `#[verus_verify]` impl block, while the methods that call
un-spec'd external flag helpers (`from_raw_value` / `into_raw_value`) stay in a plain
impl block (otherwise `#[verus_verify]` would force-verify them and fail on the external
`*Flag::from_raw_value` calls). This is exactly the impl-split layout `pte.rs` already
uses. The manual `#[cfg_attr(verus_keep_ghost, …)]` markers were then removed.

Resulting structure (mirrors `pte.rs`):
- `#[verus_verify] impl PageDirectoryEntryFlags { fn new }` + plain
  `impl PageDirectoryEntryFlags { is_present, …, from_raw_value, into_raw_value }`.
- plain `impl PageDirectoryEntry { const SIZE }` + `#[verus_verify] impl
  PageDirectoryEntry { fn new }` + plain `impl PageDirectoryEntry { from_raw_value, …,
  frame_address, … }`.

No `admit`/`assume`/`external_body`/`assume_specification` exist in the pde files, so
nothing else required elimination.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-x86-pde/verification_todo.md)

None. Zero proof gaps remain. `make verify-arch` reports 47 verified, 0 errors with
cfg_gate=0, admit=0, assume=0.

## AST Consistency

- Tool: `scripts/ast_consistency.py` against the original pre-verification baseline
  `git show dev:…/pde.rs`.
- Result: `Consistent: ✅ YES (matched=23 mismatched=0 missing=0 extra=0)`.
- Zero mismatches confirmed: YES.
- `frame_address` is now byte-identical to the original single-expression body
  (`self.frame.into_raw_value() << crate::mem::FRAME_SHIFT`); the proof obligation is
  discharged by a stripped `proof! { broadcast use lemma_frame_address; }` block plus a
  `broadcast` trigger on `(raw << FRAME_SHIFT)` in `pde.proof.rs`. No exec function body
  or signature changed (semantics, time, and space complexity preserved). `#[verus_verify]`
  and `proof!` are `cfg_erase()`-based and are erased in normal `cargo build`
  (confirmed via `./z build -- check-kernel`).

## Result: PASS
