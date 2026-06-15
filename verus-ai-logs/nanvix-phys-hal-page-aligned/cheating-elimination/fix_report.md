# Cheating Elimination Report: hal-page-aligned

Module: `hal::mem::types::address::aligned::page`
Files: `page.rs`, `page.spec.rs`, `page.proof.rs`
In-scope functions: `PageAligned::into_raw_value`, `PageAligned::from_address`, `PageAligned`

## Cheating Counts (before → after)

Module-scoped (`make verify-kernel MODULE=hal::mem::types::address::aligned::page`):

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 1      | 1*    | 0          |
| assume_specification | 1      | 1*    | 0          |
| cfg-gated exec       | 1      | 1**   | 0          |

`*` = listed in `verus-ai-logs/tcb-allowed.md` (human-approved trust boundary).
`**` = ghost-only `verus!` material block, not gated-out exec code (see below).

Module verification: **1 verified, 0 errors, exit 0**.
Full kernel (`make verify-kernel`): **exit 0** (global `external_body=24
admit=0 assume=0 trusted=0 cfg_gate=9`, the accepted baseline).
Full crate (`make verify`): all crates **exit 0**, no regressions.

## Items Eliminated

None could be *removed* — both in-scope cheating items are genuine,
human-approved trust boundaries blocked by **unverified upstream `sys`/`arch`
dependencies**, not by proof difficulty in this module. Per the
**verus-constraints** escalation ladder each was investigated (vstd search,
upstream-trait inspection, in-place reproducer) and confirmed unavoidable in
scope:

- **`PageAligned::from_address` (`external_body`, page.rs).** The body calls
  `<T as Address>::is_aligned(PAGE_ALIGNMENT)`. `is_aligned` is an **unspecced**
  method of the external `sys::mm::Address` trait, `spec_addr` is an
  `uninterp spec fn`, and `arch::mem::PAGE_ALIGNMENT` is an `arch` `Alignment`
  enum constant the Verus front-end cannot translate. Body-verification would
  require a **new** `assume_specification` on the `sys` trait — a larger,
  unapproved external-bottom surface than the single `external_body`. Listed in
  `tcb-allowed.md`; kept as the documented trust boundary.

- **`<PageAligned<T> as Address>::into_raw_value` (`assume_specification`,
  page.spec.rs).** Reproduced in-place: marking the
  `impl Address for PageAligned<T>` block verified triggers the Verus front-end
  panic `vir/src/traits.rs:511:13: assertion failed: !method_impls.contains(&p)`
  — exactly as documented in `tcb-allowed.md`. The inner
  `<T as Address>::into_raw_value` is also unspecced. The `assume_specification`
  (human-approved, listed in `tcb-allowed.md`) is the necessary workaround and is
  not counted by the cheating gate.

- **cfg-gated `verus! { }` block (page.rs:230).** This wraps the ghost-only
  `View` impl and `inv` spec, which reference the ghost `spec_addr` /
  `spec_page_size` and therefore cannot exist in a normal `cargo build`. It is
  legitimate cfg-gated **ghost/spec material**, not exec code gated out to evade
  Verus. It is unchanged from the base branch and is part of the accepted global
  `cfg_gate=9` baseline.

No `admit()` or `assume()` exist anywhere in the three files (the proof file is
empty: `verus! { }`).

## Verification TODOs (`verus-ai-logs/nanvix-phys-hal-page-aligned/verification_todo.md`)

- `PageAligned::from_address` — unspecced `<T as Address>::is_aligned`,
  uninterpreted `spec_addr`, and untranslatable `arch::mem::PAGE_ALIGNMENT`
  (`error: ... PAGE_ALIGNMENT is not supported`). Removed when `sys::mm::Address`
  / `Alignment` are verified.
- `<PageAligned<T> as Address>::into_raw_value` — Verus front-end panic
  `vir/src/traits.rs:511` on the generic trait impl, plus unspecced inner
  `into_raw_value`. Removed when `sys::mm::Address` is verified.

## AST Consistency

- Checker: `ast_consistency.py --base-ref 465c1485f` → **17 functions, 1 struct
  match; 0 mismatched, 0 missing, 0 extra.**
- Zero mismatches confirmed: **YES**. All exec code is byte-for-byte equivalent
  (AST-level) to the base; only ghost/spec annotations differ.

## Result: PASS

All in-scope cheating items are human-approved trust boundaries recorded in
`verus-ai-logs/tcb-allowed.md`, blocked by unverified upstream `sys`/`arch`
dependencies and a confirmed Verus front-end bug. No `admit()`/`assume()`
remains; no non-allowed `external_body`. Module verifies (1 verified, 0 errors);
full kernel and full crate verify with exit 0; AST consistency is clean.
