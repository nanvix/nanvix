# Cheating Elimination Report: bump-allocator

## Cheating Counts (before → after)
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 2 | 2 | 0 (both TCB-allowed) |
| assume_specification | 1 | 1 | 0 (external-bottom, not a cheating item) |
| cfg-gated exec | 0 | 0 | 0 |

`make verify-bump-allocator`: **12 verified, 0 errors**.
Cheating gate: `assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.

## Items Eliminated
None required. The crate was already free of `admit()`, `assume()`, cfg-gated
exec code, and non-TCB `external_body`. The two remaining `external_body` sites
are pre-approved trust boundaries and are correctly retained:

- `FixedSizeBumpAllocator::alloc` (lib.rs:286) — `external_body`. Materializes
  `&'static mut [u8; N]` from a backend address (`usize as *mut`); unverifiable
  without a `PointsTo` for the externally-owned `BssStorage` region (mirrors
  `src/libs/raw-array`). Listed in `verus-ai-logs/tcb-allowed.md`. Retains a full
  `#[verus_spec]` (alignment + in-bounds over `bump_view`). **Allowed → kept.**
- `FixedSizeBumpAllocator::alloc_as<T>` (lib.rs:367) — `external_body`. Delegates
  to `alloc` and re-materializes the slot as `&'static mut MaybeUninit<T>`; same
  rationale, adds the `size_of`/`align_of` vs `(N, A)` guard arms. Listed in
  `tcb-allowed.md`. **Allowed → kept.**

In-scope verified functions (not external_body):
- `align_up` (lib.rs:133) — fully verified against `align_up_spec`.
- `BssStorage::as_mut_ptr` (lib.rs:204) — specified trait method
  (`ensures result as int == base_of::<Self>()`), verifies.

### `assume_specification [<usize>::div_ceil]` (lib.spec.rs:28)
Escalation ladder followed: searched the pinned vstd for a `div_ceil` spec — none
exists. This is the standard external-bottom mechanism for giving a contract to a
`core` method that vstd does not model; it is **not** flagged by the cheating gate
(`assume=0`). Eliminating it would require rewriting `align_up`'s exec body away
from `value.div_ceil(alignment)`, which is forbidden by ast-consistency and not
required by Verus. Retained as a justified trusted boundary.

## Verification TODOs (verus-ai-logs/nanvix-phys-bump-allocator/verification_todo.md)
- No remaining proof gaps. Only the two TCB-allowed trust-boundary `external_body`
  sites and the one external-bottom `div_ceil` trusted spec remain — all justified,
  none counted by the cheating gate.

## AST Consistency
- Zero mismatches confirmed: YES. `lib.rs`, `lib.spec.rs`, and `lib.proof.rs` are
  byte-identical to `verus-ai-prove-bottom-up` (empty `git diff`). No exec code was
  changed; no cfg-gated exec variants exist.

## Result: PASS
