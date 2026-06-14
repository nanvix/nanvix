# Cheating Elimination Report: bump-allocator

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 2 | 2 | 0 (both TCB-allowed) |
| assume_specification | 1 | 1 | 0 (faithful std-lib spec) |
| cfg-gated exec | 0 | 0 | 0 |

`make verify-bump-allocator` → Verus exit code **0** (verification passes).
Cheating gate: `assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.

## Items Eliminated
No disallowed cheating items were present. The module entered the
cheating-elimination phase already in a clean state (the specification and
proving phases discharged every proof obligation). Each remaining flagged item
is permitted:

- **`FixedSizeBumpAllocator::alloc` — `external_body` (lib.rs:271).**
  Registered in `verus-ai-logs/tcb-allowed.md`. The success path materializes a
  `&'static mut [u8; N]` from a backend-provided address (`usize as *mut`), an
  int-to-pointer materialization over externally-owned `BssStorage` memory that
  Verus cannot verify without a `PointsTo` permission (a True Limitation —
  `static mut` / raw-pointer materialization; mirrors `src/libs/raw-array`). The
  function is **not** contract-free: it carries a full `#[verus_spec]` pinned to
  the abstract `bump_view` (`requires bump_view(self).inv()`; `ensures` the
  returned slot is `unit_align`-aligned and fully in-bounds).

- **`FixedSizeBumpAllocator::alloc_as<T>` — `external_body` (lib.rs:348).**
  Registered in `verus-ai-logs/tcb-allowed.md`. Delegates to `alloc` and
  re-materializes the slot as `&'static mut MaybeUninit<T>` (same raw-pointer
  limitation). Its `#[verus_spec]` adds the caller-visible `size_of::<T>()`/
  `align_of::<T>()` vs `(N, A)` guard arms.

- **`<usize>::div_ceil` — `assume_specification` (lib.spec.rs:28).**
  External-bottom *std-library* contract, not a proof escape. `align_up` calls
  `value.div_ceil(alignment)`; vstd ships no spec for `div_ceil` (verified:
  `grep -rn div_ceil` over the installed vstd returns nothing — see escalation
  below), so a faithful contract is supplied exactly as vstd's own
  `std_specs/num.rs` does for other integer methods. The contract is sound for
  unsigned operands: `result == (x + y - 1) / y` for `y != 0`, no overflow. The
  cheating gate does not count it (`assume=0`).

### Escalation ladder applied to `div_ceil`
1. **Search vstd** — `grep -rn "div_ceil"` across the pinned vstd source: no
   spec exists.
2. **Safe API alternative** — none; `div_ceil` is a std intrinsic with no
   Verus-visible body.
3. **Restructure to avoid the call** — would require changing the exec body
   (`value.div_ceil(alignment).checked_mul(alignment)`), which is forbidden by
   source-integrity (AST consistency). A faithful std-library `assume_specification`
   is therefore the correct, minimal, sound resolution.

## Verification TODOs (verus-ai-logs/nanvix-phys-bump-allocator/verification_todo.md)
None. There are zero remaining proof gaps: no `admit()`, no `assume()`. All
proof lemmas in `lib.proof.rs` (`lemma_geometry`, `lemma_exhausted_boundary`,
`lemma_alloc_transition`) are fully discharged and Verus verifies the crate with
exit code 0. The two `external_body` functions are deliberate, TCB-allowed trust
boundaries (raw-pointer / `static mut` materialization), not unproven gaps.

## AST Consistency
- Tool: `scripts/ast_consistency.py --base-ref verus-ai-prove src/libs/bump_allocator/src/lib.rs count`
- Result: **✅ Consistent: 12 functions, 7 structs match.**
- `git diff verus-ai-prove -- src/libs/bump_allocator/src/lib.rs` is empty — exec
  source is byte-identical to the base branch. No exec logic, signature, comment,
  or structure changed.
- Zero mismatches confirmed: **YES**

## Result: PASS
