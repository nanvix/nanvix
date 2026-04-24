# Trust Boundaries: kpool

This file records genuine external-bottom trust boundaries used by the kpool module.
These are calls to code outside the verification scope whose implementation cannot be
verified (hardware, FFI, std library internals).

## Dependency Trust Boundaries

These `external_body` items are in dependency modules, not in kpool itself.
Kpool consumes their contracts but does not own the trust decision.

### `pa_into_raw` — STDLIB_WRAPPER

- **Function**: `pa_into_raw` (`hal/mem/types/address/frame.rs:94`)
- **Trust item**: `#[verus_verify(external_body)]`
- **Body**: `pa.into_raw_value()` — a single method call on `PageAligned<PhysicalAddress>`
- **Why needed**: Verus cannot resolve the generic trait method `.into_raw_value()`
  on `PageAligned<PhysicalAddress>` (monomorphic `assume_specification` not possible
  for this generic chain). The wrapper isolates the trust to a single-line call.
- **Spec**: `ensures ret as int == pa@` — faithful to the semantics of `into_raw_value`.
- **Classification**: STDLIB_WRAPPER

## kpool-Specific Trust Boundaries

None. All four `Inner` methods (`new`, `alloc`, `alloc_range`, `free`) that were
previously `external_body` are now fully body-verified.
