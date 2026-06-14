# Cheating Elimination Report: sys-address-mod

## Cheating Counts (before → after)

Measured with `make verify-sys` (whole `sys` crate cheating scan).

| Item                 | Before | After | Eliminated |
|----------------------|--------|-------|------------|
| admit()              | 0      | 0     | 0          |
| assume()             | 0      | 0     | 0          |
| external_body        | 0      | 0     | 0          |
| assume_specification | 0      | 0     | 0          |
| cfg-gated exec       | 1      | 0     | 1          |

Final `make verify-sys` summary:
`assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0` → **status: CLEAN**
(`6 verified, 0 errors`).

## Items Eliminated

- **cfg-gated exec ×1 — `src/libs/sys/src/sys/mm/alignment.rs:151`.**
  The spec-only block `#[cfg(verus_keep_ghost)] verus! { pub open spec fn
  spec_align_value(..) -> int { .. } }` carried a redundant `#[cfg(verus_keep_ghost)]`
  gate that the cheating scanner flags as cfg-gated exec code. This was the **sole**
  cheating item in the entire `sys` crate (the in-scope target functions
  `is_aligned`, `into_raw_value`, `from_raw_value` and their `VirtualAddress` impls in
  `address/mod.rs` + `address/virt.rs` were already free of `assume`/`admit`/`external_body`).

  **Fix:** removed the redundant `#[cfg(verus_keep_ghost)]` attribute so the block is a
  bare `verus! { … }` block. This matches the established same-crate convention in the
  address module itself — `address/virt.rs:319` already declares its `impl View for
  VirtualAddress { closed spec fn view(&self) -> int { … } }` inside a **bare** `verus!`
  block with no cfg gate. The `verus!` macro strips spec items in non-verus builds, so the
  gate was never required; `alignment.rs` already imports `::vstd::prelude::*` (line 33),
  exactly like `virt.rs`. `spec_align_value` is referenced only from spec code
  (`address/mod.spec.rs::spec_addr_is_aligned`, kernel `page.spec.rs`), so no exec path is
  affected. Body, signature, and visibility of `spec_align_value` are byte-identical.

## Verification TODOs (verus-ai-logs/nanvix-phys-sys-address-mod/verification_todo.md)

- None. Zero proof gaps remain: no `admit()`, no `assume()`, no `external_body`, no
  `assume_specification`. `make verify-sys` reports `6 verified, 0 errors`, status CLEAN.

## AST Consistency

- **`alignment.rs`** — `ast_consistency.py` (vs base `verus-ai-prove-bottom-up`):
  `✅ Consistent: 12 functions, 0 structs match`. The cfg-attribute removal changes **no**
  exec AST (the affected block is spec-only); zero exec mismatches.
- **`address/mod.rs`** — `✅ Consistent: 0 functions, 0 structs match` (trait-only file,
  unchanged).
- **`address/virt.rs`** — unchanged by this task (`git diff` against the pre-task START
  commit is empty / byte-identical). The summary tool reports 3 "MISMATCH" rows
  (`align_up`, `align_down`, `is_aligned`); these are a **pre-existing tool name-collision
  artifact**: each name has two definitions in the file — an inherent method
  (`-> Option<Self>` / `-> Self` / `-> bool`) and the `Address` trait method
  (`-> Result<…, Error>`) — and the name-keyed matcher pairs the wrong overloads. Querying
  each name directly returns `MATCH` for `align_up`/`is_aligned`; the `align_down` "diff"
  is simply the trait overload's body. Not introduced by, nor related to, this change.
- **Required-change evidence:** the cfg removal is required to drive `cfg_gate` to 0 (the
  only blocker for a clean `make verify-sys`); it preserves semantics, time complexity, and
  space complexity (spec-only relocation of a gating attribute; precedent: `virt.rs`).
- Zero mismatches confirmed: **YES** (no exec-AST changes; the `virt.rs` rows are a
  pre-existing, unrelated tool artifact on untouched code).

## Full-crate verification

- `make verify-sys` → `6 verified, 0 errors`, **status: CLEAN**.
- `make verify` (whole workspace) → all crates `0 errors`
  (sys 6, bitmap 47, arch 70, nanvix-slab 35, kernel 76). No regressions. The kernel-crate
  cheating counters reported by `make verify` are pre-existing and out of scope for this
  task (separate kernel modules; several listed in `verus-ai-logs/tcb-allowed.md`); the
  `sys/mm/alignment.rs` file is not compiled into the kernel crate, so this change cannot
  affect them.

## Result: PASS
