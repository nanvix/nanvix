# Trust Boundaries: frame module

All entries are external-bottom trust boundaries — calls to code outside the
verification scope whose implementation cannot be verified by Verus.

---

## 1. `ExFrameNumber` external_type_specification

- **File:** `frame.spec.rs:10-12`
- **Trust item:** `#[verifier::external_type_specification]` + `#[verifier::external_body]`
- **Classification:** `EXTERNAL_TYPE`
- **Justification:** `FrameNumber` is defined in the `arch` crate. Verus needs a
  type specification to reason about it. The type itself is a newtype wrapper
  around `usize` in the arch crate; its internal representation is opaque to the
  kernel verification scope.
- **Reproducer:** N/A — external_type_specification is the standard mechanism
  for bringing external types into Verus scope.

---

## 2. `assume_specification[FRAME_SIZE]`

- **File:** `frame.spec.rs:18-23`
- **Trust item:** `assume_specification` on `::arch::mem::FRAME_SIZE`
- **Classification:** `EXTERNAL_CONST`
- **Justification:** `FRAME_SIZE` is a constant defined in the `arch` crate
  (value = 4096). Verus cannot evaluate external constants. The spec asserts
  `result == spec_page_size()`, `result > 0`, and `result >= 2`, which match
  the actual constant value.
- **Reproducer:** Verus cannot resolve constants from external crates without
  `assume_specification`.

---

## 3. `frame_addr_to_bitmap_index`

- **File:** `frame.rs:63-70`
- **Trust item:** `#[verus_verify(external_body)]`
- **Classification:** `STDLIB_WRAPPER`
- **Justification:** Wraps `self_.into_frame_number().into_raw_value()` — a
  method chain on `FrameAddress` (a `PageAligned<PhysicalAddress>`) that
  involves generic `Deref` trait dispatch. Verus cannot express
  `assume_specification` on generic trait methods (`Deref::deref` for
  `PageAligned<T>`). The wrapper body is a single expression.
- **Spec:** `requires self_.inv()`, `ensures ret as int == self_@ / spec_page_size()`
- **Reproducer:** `assume_specification` cannot match generic method signatures
  like `<PageAligned<T> as Deref>::deref`.

---

## 4. `bitmap_index_to_frame_addr`

- **File:** `frame.rs:74-89`
- **Trust item:** `#[verus_verify(external_body)]`
- **Classification:** `STDLIB_WRAPPER`
- **Justification:** Wraps `FrameNumber::from_raw_value(index)` followed by
  `FrameAddress::from_frame_number(frame_number)` — a two-step conversion chain
  on arch crate types. `FrameNumber::from_raw_value` returns `Option` (partial
  function on arch type), and `FrameAddress::from_frame_number` involves generic
  `PageAligned` construction. Neither has vstd specs.
- **Spec:** `requires frame_addr_of(index as int) <= usize::MAX as int`,
  `ensures ret.is_ok()`, `ret matches Ok(fa) ==> fa@ == index as int * spec_page_size() && fa.inv()`
- **Reproducer:** Same as #3; arch crate types use generic traits.

---

## 5. `page_aligned_pa_to_bitmap_index`

- **File:** `frame.rs:93-100`
- **Trust item:** `#[verus_verify(external_body)]`
- **Classification:** `STDLIB_WRAPPER`
- **Justification:** Wraps `self_.into_frame_number().into_raw_value()` on a
  `PageAligned<PhysicalAddress>`. Same limitation as #3 (generic Deref).
- **Spec:** `requires self_.inv()`, `ensures ret as int == self_@ / spec_page_size()`
- **Reproducer:** Same as #3.

---

## 6. `region_start_frame_number`

- **File:** `frame.rs:104-111`
- **Trust item:** `#[verus_verify(external_body)]`
- **Classification:** `STDLIB_WRAPPER`
- **Justification:** Wraps `region.start().into_frame_number().into_raw_value()`
  on a `TruncatedMemoryRegion<PhysicalAddress>`. The `start()` method returns a
  `PageAligned<PhysicalAddress>`, then the same generic Deref chain as #3.
- **Spec:** `requires region.inv()`, `ensures ret as int == region@.start / spec_page_size()`
- **Reproducer:** Same as #3.

---

## 7. `region_size_raw`

- **File:** `frame.rs:115-121`
- **Trust item:** `#[verus_verify(external_body)]`
- **Classification:** `STDLIB_WRAPPER`
- **Justification:** Wraps `region.size()` on a `TruncatedMemoryRegion<PhysicalAddress>`.
  `TruncatedMemoryRegion::size()` is defined in the HAL crate; Verus cannot
  verify its body (external to the kernel crate's verification scope).
- **Spec:** `ensures ret as int == region@.size`
- **Reproducer:** External HAL method; no vstd spec available.

---

## 8. `region_start_raw`

- **File:** `frame.rs:125-132`
- **Trust item:** `#[verus_verify(external_body)]`
- **Classification:** `STDLIB_WRAPPER`
- **Justification:** Wraps `region.start().into_raw_value()` via generic Deref.
  Same limitation as #3.
- **Spec:** `requires region.inv()`, `ensures ret as int == region@.start`
- **Reproducer:** Same as #3.
