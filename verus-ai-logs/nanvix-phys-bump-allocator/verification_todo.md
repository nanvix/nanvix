# Verification TODO — bump-allocator (cheating-elimination phase)

No remaining proof gaps. `make verify-bump-allocator` reports **12 verified, 0
errors**, with the cheating gate at `assume=0 admit=0 trusted=0 no_decreases=0
cfg_gate=0` and `external_body=2`.

Both remaining `external_body` sites are **TCB-allowed trust boundaries**, listed
in `verus-ai-logs/tcb-allowed.md`, not proof gaps:

- `FixedSizeBumpAllocator::alloc` — materializes `&'static mut [u8; N]` from a
  backend-provided address (`usize as *mut`). Unverifiable without a `PointsTo`
  permission for the externally-owned `BssStorage` region (mirrors `raw-array`).
  Carries a full `#[verus_spec]` pinning alignment + in-bounds over `bump_view`.
- `FixedSizeBumpAllocator::alloc_as<T>` — delegates to `alloc`, re-materializes the
  slot as `&'static mut MaybeUninit<T>`. Same rationale; adds the
  `size_of::<T>()`/`align_of::<T>()` vs `(N, A)` guard arms.

The `assume_specification [<usize>::div_ceil]` in `lib.spec.rs` is an
external-bottom trusted contract for a `core` method **not modeled by vstd**
(confirmed: no `div_ceil` spec exists in the pinned vstd). It is not counted by
the cheating gate and is the standard mechanism for speccing std callees; it
cannot be eliminated without rewriting `align_up`'s exec body (forbidden by
ast-consistency, and not required by Verus).
