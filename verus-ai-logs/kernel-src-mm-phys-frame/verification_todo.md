# Verification TODOs: frame module

Items that are verifiable in principle but cannot be verified with current
Verus capabilities. These are NOT trust boundaries — they are proof gaps
that should be resolved when Verus adds support for the blocking constructs.

---

## 1. `assume_specification[init]`

- **Function:** `init` (`frame.rs:1163-1180`)
- **Spec location:** `frame.spec.rs:38-42`
- **Status:** UNPROVEN
- **Trust item:** `assume_specification` on module-owned function
- **Current spec:** Vacuous — `result.is_ok() || result.is_err()` (trivially true)
- **Missing guarantees:**
  - Singleton is initialized after `Ok` return
  - Bitmap state is preserved in the singleton
  - Double-init is rejected with `Err`
- **Attempts:**
  - Cannot use `#[verus_verify(external_body)]` — body uses `MaybeUninit::write()`
    which Verus cannot compile even inside `external_body`
  - Cannot verify body — uses `static mut`, `unsafe`, `MaybeUninit::write()`,
    `AtomicBool::store()`, none of which have vstd support
- **Blocker:** Verus cannot compile functions containing `MaybeUninit::write()`.
  Additionally, `static mut` access and `unsafe` blocks are not supported.
  Even `external_body` is insufficient because Verus fails at the parsing/
  compilation stage before reaching the verification stage.

---

## 2. `pub alloc()` — singleton wrapper

- **Function:** `alloc` (`frame.rs:1184-1195`)
- **Status:** UNPROVEN (body verified indirectly via Inner::alloc)
- **Trust item:** `#[verus_verify(external_body)]`
- **Current spec:** `ensures match result { Ok(frame) => frame.inv(), Err(_) => true }`
- **Missing guarantees:**
  - Full state-transition spec (available on Inner::alloc but lost at singleton boundary)
  - Singleton is initialized (precondition delegation to instance())
- **Attempts:**
  - Body calls `instance().alloc()` where `instance()` uses `static mut` +
    `unsafe` + `MaybeUninit::assume_init_mut()` + `AtomicBool::load()`
  - Inner::alloc IS fully verified with rich pre/postconditions
  - Cannot propagate Inner::alloc's spec through instance() because Verus
    cannot reason about static mutable singletons
- **Blocker:** Verus does not support `static mut` or `MaybeUninit::assume_init_mut()`.
  The `instance()` accessor function is unverifiable.

---

## 3. `pub free()` — singleton wrapper

- **Function:** `free` (`frame.rs:1199-1213`)
- **Status:** UNPROVEN (body verified indirectly via Inner::free)
- **Trust item:** `#[verifier::external_body]`
- **Current spec:** `requires frame.inv()`, `ensures result.is_ok() || result.is_err()`
- **Missing guarantees:** Full state-transition spec from Inner::free
- **Attempts:** Same as #2
- **Blocker:** Same as #2 — singleton pattern uses unsupported constructs.
  Note: uses `verus!` syntax (not attribute style) because `no_unwind` is
  required by `Drop::drop` and attribute syntax doesn't support `no_unwind`.

---

## 4. `pub book()` — singleton wrapper

- **Function:** `book` (`frame.rs:1217-1227`)
- **Status:** UNPROVEN (body verified indirectly via Inner::book)
- **Trust item:** `#[verus_verify(external_body)]`
- **Current spec:** `requires phys_addr.inv()`, `ensures result.is_ok() || result.is_err()`
- **Missing guarantees:** Full state-transition spec from Inner::book
- **Attempts:** Same as #2
- **Blocker:** Same as #2.

---

## 5. `pub alloc_range()` — singleton wrapper

- **Function:** `alloc_range` (`frame.rs:1231-1241`)
- **Status:** UNPROVEN (body verified indirectly via Inner::alloc_range)
- **Trust item:** `#[verus_verify(external_body)]`
- **Current spec:** `requires region.inv()`, `ensures result.is_ok() || result.is_err()`
- **Missing guarantees:** Full state-transition spec from Inner::alloc_range
- **Attempts:** Same as #2
- **Blocker:** Same as #2.

---

## Summary

All 5 items share a common root cause: Verus cannot reason about the
singleton pattern (`static mut` + `MaybeUninit` + `AtomicBool`). The inner
methods (`Inner::alloc`, `Inner::free`, `Inner::book`, `Inner::alloc_range`)
are fully body-verified with rich specifications. The proof gap exists only
at the singleton accessor boundary.

**Resolution path:** When Verus adds support for `static mut` or provides a
verified singleton abstraction, these items can be resolved by:
1. Writing `assume_specification` for `AtomicBool::load/store` and
   `MaybeUninit::assume_init_mut/write`
2. Verifying `instance()` and `init()` with those specs
3. Removing `external_body` from the pub wrappers and propagating
   Inner method specs through the singleton accessor
