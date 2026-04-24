# Cheating Elimination Report: upool

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 0 | 0 | 0 |
| cfg-gated exec | 0 | 0 | 0 |

**Note:** The one `#[cfg(not(verus_keep_ghost))]` in upool.rs:108 gates the
`error!` logging macro inside `UserFrame::drop`. Per verus-constraints, logging
macros are explicitly allowed to be cfg-gated (non-semantic item). This is not
counted as cheating.

## Items Eliminated

No cheating items were present — no elimination was needed.

## Trust Boundaries (trust.md)

None. The upool module has no external-bottom trust boundaries. All six
functions (`UserFrame::new`, `address`, `leak`, `drop`, `Upool::new`,
`Upool::alloc`) are fully body-verified.

## Verification TODOs (verification_todo.md)

None. All functions verify cleanly with zero proof gaps.

## AST Consistency

- Zero mismatches confirmed: NO (1 mismatch on `UserFrame::drop`)

### UserFrame::drop Mismatch Analysis

Three differences from the `exp` baseline:

1. **`opens_invariants none` / `no_unwind` annotations** — Verus requires these
   on `Drop` impls. They are ghost-level spec annotations, not exec code
   changes. Erased during normal `cargo build` via the `verus!{}` macro.

2. **`e` → `_e` variable rename** — Suppresses unused-variable warning when the
   `error!` logging macro is cfg-gated out. Pre-approved deviation: variable
   rename with identical semantics, panic behavior, time/space complexity.

3. **`#[cfg(not(verus_keep_ghost))]` on `error!` macro** — Allowed per
   verus-constraints (logging macros are non-semantic items).

All three are justified Verus adaptations that preserve semantics.

## Result: PASS
