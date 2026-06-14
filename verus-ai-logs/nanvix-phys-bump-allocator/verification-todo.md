# Verification TODO — `bump_allocator` (`src/libs/bump_allocator/src/lib.rs`)

Items deferred from the specification phase. Nothing here is a failed proof: the
crate verifies with `0 errors`. These record the trust boundaries and the
proving-phase modeling work that would eliminate them.

## `FixedSizeBumpAllocator::alloc` / `alloc_as<T>`
- Status: SPECIFIED + TRUSTED (`#[verus_verify(external_body)]`, registered in
  `verus-ai-logs/tcb-allowed.md`).
- Reason: the success path materializes a `&'static mut` slot from a backend
  raw address (`usize as *mut`) over externally-owned `BssStorage` memory — a
  raw-memory op Verus cannot verify without a `PointsTo` permission. See
  `verus-unsupported.md`.
- Caller contracts (already attached as `#[verus_spec]`, checked by every caller):
  `Ok` ⟹ slot is `A`-aligned, in-bounds (`base <= a && a + N <= base + storage_size`);
  `alloc_as` additionally `Ok ⟹ size_of::<T>() == N && align_of::<T>() <= A`,
  `Err(SizeMismatch) ⟹ size_of::<T>() != N`,
  `Err(AlignmentMismatch) ⟹ align_of::<T>() > A`.
- Deferred (proving phase): replace `external_body` with a verified body backed by
  a `vstd::atomic_ghost` invariant for the `next_slot` cursor plus a `PointsTo`
  permission for the `BssStorage` region, so the `v -> v'` transition
  (`allocated + 1`, cross-call slot uniqueness — already stated in
  `lemma_alloc_transition` / `lemma_geometry`) is discharged against the real body.

## `BssStorage::as_mut_ptr`
- Status: SPECIFIED (`#[verus_spec]`, `ensures result as int == base_of::<Self>()`).
- `base_of::<S>()` is an uninterpreted ghost constant (a static's address is opaque
  to Verus). The ensures encodes the one safe, non-tautological fact callers need —
  **stability**: every call returns the same address, to which `BumpView::base` is
  pinned. The `A`-alignment / `>= STORAGE_SIZE`-writable duties are the unsafe TCB
  contract of the backend and cannot be stated at the trait level (`A` is not a
  `BssStorage` parameter). `make verify` / `make verify-kernel` both pass.
