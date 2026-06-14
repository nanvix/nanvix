# Verification TODO — `bump_allocator` (cheating-elimination phase)

## Remaining proof gaps: NONE

`make verify-bump-allocator` verifies the crate with Verus exit code **0**.

- `admit()`: 0
- `assume()`: 0
- cfg-gated exec: 0

All proof lemmas in `lib.proof.rs` are fully discharged:
- `lemma_geometry` — alignment / in-bounds / uniqueness, proved.
- `lemma_exhausted_boundary` — exhaustion boundary, proved.
- `lemma_alloc_transition` — single-slot advance + invariant preservation, proved.

## Permitted trust boundaries (NOT proof gaps)

These are registered in `verus-ai-logs/tcb-allowed.md` and are genuine Verus
True Limitations (int-to-pointer materialization over externally-owned
`static mut` / `BssStorage` memory), not unproven obligations:

- `FixedSizeBumpAllocator::alloc` — `external_body`, full `#[verus_spec]` contract.
- `FixedSizeBumpAllocator::alloc_as` — `external_body`, full `#[verus_spec]` contract.

The `assume_specification[<usize>::div_ceil]` (lib.spec.rs) is a faithful
std-library contract (vstd ships no `div_ceil` spec); it is external-bottom for a
std intrinsic, not a deferred proof obligation.
