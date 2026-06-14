# Verus Unsupported Constructs — `bump_allocator` (`src/libs/bump_allocator/src/lib.rs`)

Genuine Verus front-end limitations encountered while specifying the in-scope
exec functions. These motivate the **trust-boundary** treatment of `alloc` /
`alloc_as`: both materialize a `&'static mut` slot from a backend-provided raw
address (`usize as *mut`) over memory owned by the external `BssStorage` backend,
for which there is no `PointsTo`/`PointsToRaw` permission source. That is a
raw-memory operation Verus cannot verify, identical in nature to the
`src/libs/raw-array` pattern. Both functions are therefore registered in
`verus-ai-logs/tcb-allowed.md` and carry `#[verus_verify(external_body)]` plus a
full caller-facing `#[verus_spec]` contract. `align_up` and the `BumpView` model
are fully verified (bodies checked), and the crate verifies with `0 errors`.

## 1. Raw-pointer materialization from an integer address

- **`alloc`** (`Ok(unsafe { &mut *(ptr as *mut [u8; N]) })`):
  ```
  error: Verus does not support this cast: `usize` to `*mut [u8; N/#0]`
  ```
- **`alloc_as`** (`Ok(unsafe { &mut *(slot.as_mut_ptr() as *mut MaybeUninit<T>) })`):
  ```
  error: The verifier does not yet support the following Rust feature:
  dereferencing a raw pointer. Currently, Verus only supports raw pointers
  through the permissioned raw_ptr interface.
  ```

`vstd::raw_ptr` is the only supported route and requires a `PointsTo` permission
for the memory. That memory is owned by the external `BssStorage` backend
(`S::as_mut_ptr()`), so no permission source exists in this crate. Fabricating one
is exactly the unsafe `BssStorage` contract — the external-bottom trust boundary
recorded in `tcb-allowed.md`. The returned slot's abstract address is therefore
modeled by the uninterpreted ghost `slot_ref_addr(slot)` (a Verus `&mut T` carries
no spec-readable address; only raw pointers expose `.addr()`), over which the
`ensures` states the alignment / in-bounds guarantees.

## 2. `core::sync::atomic::AtomicUsize` value is not spec-readable

Reading the dynamic `allocated` count from the `next_slot: AtomicUsize` cursor
inside a deterministic `spec fn view()` is impossible: `load` is an `exec` method,
uncallable from `spec`, and vstd's `std_specs/atomic.rs` explicitly provides
"NO support for reasoning about the values inside the atomics." The abstract
`bump_view(self)` accessor is therefore an `uninterp spec fn` (a mechanical
consequence of the `external_body` `AtomicUsize` type, mirroring `raw-array`'s
uninterpreted `view`). `inv()` pins `base/stride/unit_size/unit_align/capacity/
storage_size` to the type-level constants; the `v -> v'` cross-call transition
(`allocated + 1`, slot uniqueness) is captured by `lemma_alloc_transition` and
deferred to the proving-phase ghost token (see `view_design.md` modeling note and
`verification-todo.md`).

## 3. `usize::div_ceil` has no vstd spec

`align_up` calls `usize::div_ceil`, for which vstd provides no specification
(`grep -rn div_ceil` over the vstd source → 0 hits). A legitimate std-library
external-bottom `assume_specification` is supplied in `lib.spec.rs`
(`requires y != 0`, `ensures result as int == (x + y - 1) / y`), faithful to the
documented unsigned ceiling-division semantics and zero-divisor panic.

## Consequence

`align_up` and the `BumpView` abstract model verify cleanly (bodies checked).
`alloc` / `alloc_as` are sound trust boundaries (`external_body`, TCB-registered)
carrying their full caller contracts; the only deferred work is replacing the
`external_body` body with a `vstd::atomic_ghost` + `PointsTo` verified body in the
proving phase (tracked in `verification-todo.md`).
