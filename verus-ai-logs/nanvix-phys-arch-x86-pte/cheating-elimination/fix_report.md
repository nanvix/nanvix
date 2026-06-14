# Cheating Elimination Report: arch-x86-pte

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 2 | 0 | 2 |

Counts are module-scoped (`x86::mem::paging::pte`). After the fix the module
reports `✅ No cheating detected in module x86::mem::paging::pte`.

(Crate-global residuals — `external_body=3`, `cfg_gate=2` — are entirely
out-of-scope: the 3 `external_body` are `invlpg`, `table::read`, `table::write`,
all listed in `verus-ai-logs/tcb-allowed.md`; the 2 `cfg_gate` are in `pde.rs`
lines 83/307, owned by the sibling `arch-x86-pde` task. None were introduced or
touched by this work.)

## Items Eliminated
- **`PageTableEntryFlags::new` — cfg-gated exec** (`pte.rs:85`):
  `#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]`.
  This per-method attribute is the only way to supply the
  `verus_impl_method_marker` allow that `#[verus_spec]` needs on an
  *associated function* (no `&self`) whose impl is **not** `#[verus_verify]`.
  Removing it outright breaks the build (`cannot find function 'new' in this
  scope` — the verus_spec proxy loses its impl context). Eliminated by adopting
  the established in-directory idiom (`table.rs`): the marker is auto-generated
  by `#[verus_verify]` on the impl. `new` was moved into its own
  `#[verus_verify] impl PageTableEntryFlags { … }` block; the remaining methods
  (`from_raw_value`, `into_raw_value`, … which call `external` flag helpers and
  must stay verus-ignored) keep their plain `impl` block.
- **`PageTableEntry::new` — cfg-gated exec** (`pte.rs:307`): identical situation
  and identical fix — `new` moved into a dedicated `#[verus_verify] impl
  PageTableEntry { … }` block; `SIZE` and the external-calling methods stay in
  their plain `impl` blocks.

### Why splitting the impl (not annotating the whole impl)
Putting `#[verus_verify]` on the *original* impl forces Verus to verify every
method body, including `from_raw_value`/`into_raw_value`, which call
`flags::*::from_raw_value`/`into_raw_value` — functions declared outside
`verus!`/marked `external`. That yields `cannot use function … which is ignored
because it is … external` (16 errors). Splitting confines `#[verus_verify]` to
the two in-scope `new` functions, leaving the external-calling methods ignored
exactly as before. Multiple `impl` blocks for one type are semantically
identical in Rust (they are merged), so exec semantics, time complexity, and
space complexity are unchanged.

## Verification TODOs (verus-ai-logs/nanvix-phys-arch-x86-pte/verification_todo.md)
None. The four in-scope functions verify with real contracts (6 verified,
0 errors). No `admit()`/`assume()` and no deferred proof gaps remain.

## AST Consistency
- Zero mismatches confirmed: YES
  (`ast_consistency.py <START:pte.rs> <current> count` →
  `✅ Consistent: 23 functions, 2 structs match`). The only changes are the two
  removed cfg-gated `allow` attributes and the impl-block split — both
  exec-invisible (impl merging). No pre-approved-deviation rewrites were needed.

## Result: PASS
