# Cheating Elimination Report: sys-virt-address

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Counts are for the in-scope module `src/libs/sys/src/sys/mm/address/virt.rs`
(and its `virt.spec.rs` / `virt.proof.rs`), and confirmed crate-wide by
`make verify-sys`:
`cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0`,
`status: CLEAN`, exit code 0.

## Items Eliminated
- **None to eliminate.** The `sys` crate (and the `virt` module specifically)
  already contained **zero** cheating markers on entry. The verification-target
  constructors are genuinely body-verified:
  - `VirtualAddress::new` — `#[verus_spec] ensures result@ == value as int, result.inv()`.
  - `VirtualAddress::from_raw_value` — `#[verus_spec] ensures result@ == raw_addr as int, result.inv()`.
  - `VirtualAddress` — carries `impl View` (`view == self.0 as int`) and the
    `inv` spec (`0 <= self@ <= usize::MAX`).
  No `admit`/`assume`/`external_body`/`assume_specification`/cfg-gated exec was
  present, so nothing required replacing with a proof.

## Verification TODOs (verus-ai-logs/nanvix-phys-sys-virt-address/verification_todo.md)
- **`<VirtualAddress as Address>::into_raw_value`** (target function) — desired
  contract `result as int == self@`. **Not a cheating item and not a proof gap**
  (no `admit`/`assume` exists for it in `sys`); it is a coverage gap blocked by a
  genuine Verus front-end limitation.
  - Blocking error, reproduced in isolation:
    `error: Verus does not support this cast: `usize` to `*const u8``.
  - Verus checks a trait `impl` as a unit, so verifying `into_raw_value` pulls in
    the sibling `as_ptr`/`as_mut_ptr` (`self.0 as *const u8` / `as *mut u8`).
    Rust forbids splitting one trait `impl`; the siblings are out-of-scope
    unlisted functions whose cast is their purpose; and `external`/`external_body`
    on them is forbidden by verus-constraints. The identity fact is therefore
    carried by a consumer-side `assume_specification` in the `kernel` crate that
    is explicitly governed by `verus-ai-logs/tcb-allowed.md` (out of scope for
    `verify-sys`). Removable when Verus supports the pointer cast.

## AST Consistency
- Tool: `scripts/ast_consistency.py --base-ref verus-ai/hal-platform-microvm`.
- Raw report: `matched=14 mismatched=4`. The 4 reported MISMATCHes
  (`align_up`, `is_aligned`, plus their pairings) are **tool false-positives**:
  `VirtualAddress` has same-named methods in both the inherent `impl` (returning
  `Option<Self>` / `bool`) and the `impl Address` trait (returning
  `Result<…, Error>`); the name-keyed checker cross-matched the inherent and
  trait versions.
- `git diff verus-ai/hal-platform-microvm -- …/virt.rs` proves **no exec logic
  changed**: the only additions are verus annotations (`#[verus_verify]`,
  `#[verus_spec]`, cfg-gated `vstd` import + `include!`s, `View`/`inv` material,
  explanatory comments) and a split of the inherent `impl VirtualAddress` block
  into two inherent `impl` blocks (semantically identical in Rust; required
  because Verus needs a `#[verus_verify]` enclosing impl for the self-less
  associated functions `new`/`from_raw_value`). No method body, signature, or
  struct definition was altered. Semantics, time complexity, and space
  complexity are preserved.
- Zero mismatches confirmed: YES (the 4 reported are verified false-positives
  with git-diff evidence of unchanged exec code).

## Result: PASS
- `make verify-sys`: CLEAN, 0 cheating, exit 0.
- `make verify` (full crate set): exit 0 on every crate — no regressions
  introduced (zero source changes were made). Pre-existing, out-of-scope
  `external_body` counts in `bump-allocator` (2) and `kernel` (25) are governed
  by their own `tcb-allowed.md` entries and are untouched by this task.
