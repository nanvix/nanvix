# Bugs / Findings — `hal::mem::types::address::aligned::page` (`PageAligned<T>`)

## Fixed: duplicate `use vstd::prelude::*` broke the normal build

- **Where:** `src/kernel/src/hal/mem/types/address/aligned/page.rs`.
- **Symptom:** the verus-task scaffolding added a second
  `use vstd::prelude::*;` (alongside the pre-existing `use ::vstd::prelude::*;`),
  producing a duplicate glob import. Under the kernel's `-D warnings`
  (`unused_imports`) a normal `cargo build` / `make build` failed with
  `error: unused import: ::vstd::prelude::*`. The verus build tolerated it, so it
  only surfaced in the non-ghost (dual-compilation) build.
- **Fix:** removed the redundant scaffolding import, restoring the single
  `use ::vstd::prelude::*;` that the pre-verus revision shipped. Both `make
  build` (normal) and `make verify-kernel` now pass.
- **Auto-fixable:** yes (redundant import), fixed directly.

## Tool limitation (recorded): Verus panics when verifying a generic `Address` trait impl

- **Where:** `<PageAligned<T> as Address>::into_raw_value`
  (`impl<T: Address> Address for PageAligned<T>`).
- **Symptom:** attaching `#[verus_spec]` to the trait method requires marking the
  whole `impl Address for PageAligned<T>` verified (Verus: "In order to verify
  any items of this trait impl, the entire impl must be verified"). Adding
  `#[verus_verify]` to that generic trait impl makes the Verus front-end panic:

  ```
  thread 'rustc' panicked at vir/src/traits.rs:511:13:
  assertion failed: !method_impls.contains(&p)
  ```

- **Not a code bug.** This is a Verus front-end limitation on verifying a generic
  `impl` of an external (`#[verus_verify]`) trait one-method-at-a-time.
- **Workaround (in place):** spec the method with `assume_specification` in
  `page.spec.rs` (exact `T: Address` bound, `ensures result as int == addr@`).
  This mirrors the codebase's existing trust boundary for
  `<PageAligned<T> as Address>::from_raw_value` (`kframe.spec.rs`). Recorded in
  `tcb-allowed.md`. Removed when the `sys::mm::Address` trait is verified.

## Tool limitation (recorded): `arch` `Alignment` constant cannot be translated

- **Where:** `PageAligned::from_address` body —
  `addr.is_aligned(PAGE_ALIGNMENT)`.
- **Symptom:** body verification fails to compile under Verus with
  `error: arch::x86::mem::constants::PAGE_ALIGNMENT is not supported` — Verus
  cannot translate the `arch` `Alignment` enum constant; `<T as Address>::is_aligned`
  is also an unspecced trait method.
- **Not a code bug.** Both items live in the not-yet-verified `sys`/`arch`
  libraries (out of scope for this module).
- **Workaround (in place):** `#[verus_verify(external_body)]` +
  `#[verus_spec(...)]` on `from_address`, honoring the real caller contract as a
  trust boundary (the `FrameAddress::into_raw_value` pattern). Recorded in
  `tcb-allowed.md`. Removed when the `Address` trait and the `Alignment` encoding
  are verified.

## Note: `View` made unconditional over `T: Address`

- The skeleton's `View`/`inv` were bounded `T: Address + View<V = int>` with
  `view = self.0@`. That bound cannot reach the in-scope exec functions (generic
  over bare `T: Address`), cannot be added to those exec impls without breaking
  `region.rs`, and is unsatisfiable in a normal build (address-family `View`
  impls are `cfg(verus_keep_ghost)`-gated). The `View`/`inv` were made
  unconditional over `T: Address`, delegating `view` to a ghost
  `uninterp spec fn spec_addr<T: Address>(&T) -> int`. `view()` is `closed`, so
  no consumer observes the change. See `view_design.md` for the full rationale.
