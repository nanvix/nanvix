# Verification TODO — `bump_allocator` (`src/libs/bump_allocator/src/lib.rs`)

Functions that are in scope but cannot be body-verified in the specification
phase. They are **not** trust boundaries (absent from `tcb-allowed.md`) and are
**not** marked `external_body`; they are honest hand-offs to the proving phase.

## `FixedSizeBumpAllocator::alloc`
- Status: UNPROVEN (unannotated — body skipped by Verus)
- Blocker: Verus front-end limitations — `break <value>` and raw-pointer deref
  (`&mut *(ptr as *mut [u8; N])`). See `verus-unsupported.md`.
- Intended contract (to attach once the atomic-ghost token lands, see
  `view_design.md` section 7): on `Ok`, the returned slot is `A`-aligned,
  in-bounds, and distinct from every prior slot; `allocated` advances by one;
  on `Err`, no slot is consumed and the variant is never `SizeMismatch` /
  `AlignmentMismatch`. Encoded as `lemma_geometry`, `lemma_exhausted_boundary`,
  `lemma_alloc_transition` in `lib.proof.rs`.

## `FixedSizeBumpAllocator::alloc_as<T>`
- Status: UNPROVEN (unannotated — body skipped by Verus)
- Blocker: raw-pointer deref (`&mut *(slot.as_mut_ptr() as *mut MaybeUninit<T>)`).
  See `verus-unsupported.md`.
- Intended contract: `Ok` ⟹ `size_of::<T>() == N && align_of::<T>() <= A` plus the
  `alloc` geometry guarantees; `Err(SizeMismatch)` ⟺ `size_of::<T>() != N`;
  `Err(AlignmentMismatch)` ⟺ `size_of::<T>() == N && align_of::<T>() > A`.

## `BssStorage::as_mut_ptr`
- Status: SPECIFIED (`#[verus_spec]` with `ensures result as int == base_of::<Self>()`).
- `base_of::<S>()` is an uninterpreted ghost constant (a static's address is opaque
  to Verus). The ensures states the one safe, non-tautological fact callers need:
  **stability** — every call returns the same address, which `BumpView::base` is
  pinned to. The remaining `A`-alignment / `>= STORAGE_SIZE`-writable-bytes duties
  cannot be stated at the trait level (`A` is not a `BssStorage` parameter) and are
  the unsafe TCB contract of the backend; the kernel's unverified
  `PageTableBss::as_mut_ptr` impl trusts the ensures, and `make verify` /
  `make verify-kernel` both pass (0 errors).
