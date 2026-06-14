# Cheating Elimination Report: bump-allocator

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 2 | 2 | 0 (both TCB-allowed) |
| assume_specification | 1 | 0 | 1 |
| cfg-gated exec | 0 | 0 | 0 |

Notes:
- `external_body` count is unchanged because both instances —
  `FixedSizeBumpAllocator::alloc` and `FixedSizeBumpAllocator::alloc_as` — are
  explicitly listed in `verus-ai-logs/tcb-allowed.md`. They materialize a
  `&'static mut` reference from a backend-provided raw address, which Verus
  cannot verify without a `PointsTo` permission for the externally-owned
  `BssStorage` region (mirrors `src/libs/raw-array`). These are permitted TCB
  items, not blockers.
- The only non-allowed cheating item was the unapproved `assume_specification`
  on `usize::div_ceil`; it has been fully eliminated.

## Items Eliminated
- **`assume_specification [ <usize>::div_ceil ]`** (`lib.spec.rs`).
  This was an unapproved external-bottom trust assumption (`div_ceil` is not in
  `tcb-allowed.md` and vstd ships no spec for the intrinsic — confirmed absent
  from vstd `std_specs`). Eliminated by:
  1. Removing the `assume_specification` block from `lib.spec.rs`.
  2. Open-coding `align_up`'s ceiling division in `lib.rs` as the
     arithmetically-equivalent `value / alignment` (+1 on a non-zero remainder),
     guarded by `checked_mul` for the final stride multiply — identical
     semantics and O(1) time/space to `value.div_ceil(alignment)`.
  3. Adding `lemma_ceil_div` (`lib.proof.rs`) proving
     `(if r == 0 { qd } else { qd + 1 }) == (v + d - 1) / d` via
     `lemma_fundamental_div_mod` plus a remainder case split — this matches the
     numeric meaning encoded in `align_up_spec`.
  4. Proving the only overflow obligation inline: `qd + 1` cannot overflow
     because a non-zero remainder forces `alignment >= 2`, hence
     `qd <= value / 2 < usize::MAX`.
  Escalation ladder followed: searched vstd (no `div_ceil` spec), built an
  isolated reproducer (`/tmp/repro_align.rs`, 6 verified / 0 errors), then
  applied the equivalent rewrite.

## Verification TODOs (verus-ai-logs/nanvix-phys-bump-allocator/verification_todo.md)
- None. No remaining proof gaps; no `admit()`/`assume()`/unapproved
  `assume_specification`. `make verify-bump-allocator` reports 10 verified,
  0 errors.

## AST Consistency
- Zero mismatches confirmed: NO — one intentional, documented exec change.
  - `align_up` is a MISMATCH against `exp` because its body was rewritten to
    avoid the unspecifiable `usize::div_ceil` intrinsic.
    - Required by Verus: vstd has no `div_ceil` spec and `assume_specification`/
      `external_body` are unapproved cheating for this function, so the original
      one-liner cannot be verified as-is. Reproducer confirms the rewrite verifies.
    - Semantics preserved: open-coded ceiling division is value-equal to
      `div_ceil`; `cargo build` succeeds and all 3 unit tests + 1 doctest pass.
    - Complexity preserved: O(1) time, O(1) space (a few integer ops, no loops).
    - Documented in-source with a `VERUS DEVIATION` comment.
  - All other 11 functions and 7 structs: MATCH.

## Result: PASS
