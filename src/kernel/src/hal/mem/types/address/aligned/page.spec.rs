// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PageAligned<T> — Specifications
//
// `PageAligned<T>` is a validated newtype wrapping a memory address (`int`)
// carrying the static guarantee that the address lies on a page boundary. Its
// abstract value is exactly that address (`PageAligned@ : int`, defined by the
// `View` impl in `page.rs`); the page-alignment fact is the property `inv()`
// over that value.
//
// `spec_addr` is the ghost projection of an arbitrary `T: Address` to its
// abstract address (`int`). It exists for *every* `T: Address` so that the
// `View` impl for `PageAligned<T>` — and hence the `@`-based contracts of the
// in-scope exec functions (`into_raw_value`, `from_address`) — can be stated
// for a bare `T: Address`, without the `T: View<V = int>` bound that would
//   * be unsatisfiable in a normal `cargo build` (the address-family `View`
//     impls are all `cfg(verus_keep_ghost)`-gated), and
//   * not be available on the generic exec `impl<T: Address> ...` blocks (and
//     would break `region.rs`, which uses `PageAligned<T>: Address` for a bare
//     `T: Address`).
// It is left uninterpreted here; the exec contracts pin it operationally
// (`into_raw_value` returns exactly `self@`), exactly as the sibling
// `FrameAddress::into_raw_value` trust boundary draws the same newtype-identity
// fact for the concrete frame address. It is `cfg(verus_keep_ghost)`-gated
// (ghost-only), so it does not exist in a normal build.

verus! {

// Ghost projection of any address to its abstract value (`int`).
pub uninterp spec fn spec_addr<T: Address>(addr: &T) -> int;

// `<PageAligned<T> as Address>::into_raw_value` is a method of the external
// `sys::mm::Address` trait. A trait-impl method cannot be body-verified in place
// without marking the whole `impl Address for PageAligned<T>` verified, which
// pulls every sibling method into scope and currently triggers a Verus
// front-end limitation (`vir/src/traits.rs` assertion). It is therefore specced
// here with `assume_specification`, mirroring the trust boundary the codebase
// already draws for `<PageAligned<T> as Address>::from_raw_value`
// (`kframe.spec.rs`) and `::arch::mem::PAGE_SIZE` (`frame.rs`). The bound is
// exactly `T: Address` (matching the external signature), and the `@`-based
// contract is expressible because `PageAligned<T>: View` now holds for every
// `T: Address`. The real proof obligation is the inner
// `<T as Address>::into_raw_value` newtype identity, discharged when the
// `Address` trait itself is verified.
//
// Pure newtype identity projection: the returned raw `usize` is exactly the
// abstract address. Callers' in-page offset math and page walking require an
// identity projection (no masking/shifting).
pub assume_specification<T: Address> [
    <PageAligned<T> as Address>::into_raw_value
](addr: PageAligned<T>) -> (result: usize)
    ensures
        result as int == addr@,
;

// `PageAligned::from_address` is a partial, identity-preserving, validating
// constructor. Its body checks page alignment via
// `<T as Address>::is_aligned(PAGE_ALIGNMENT)`, where `PAGE_ALIGNMENT` is an
// `arch` `Alignment` enum constant the Verus front-end cannot translate, and
// `is_aligned` is an unspecced `Address` trait method. The function therefore
// cannot be body-verified in place without speccing those upstream `sys`/`arch`
// items (out of scope), so it is specced with `assume_specification` — the same
// trust boundary the codebase already draws at the `sys`/`arch` library edge.
// The contract is the real caller-facing guarantee and is discharged when the
// `Address` trait and the `Alignment` encoding are verified.
//
// On success the wrapped address is unchanged (`p@ == spec_addr(&addr)`) and the
// page-alignment invariant is established (`p.inv()`); the constructor validates,
// it never rounds/normalizes. On failure the input was not page-aligned (value
// type: no side effect). Success holds iff the input address is page-aligned
// (stated both ways for liveness).
pub assume_specification<T: Address> [
    PageAligned::<T>::from_address
](addr: T) -> (result: Result<PageAligned<T>, Error>)
    ensures
        match result {
            Ok(p) => p@ == spec_addr(&addr) && p.inv(),
            Err(_) => spec_addr(&addr) % crate::hal::mem::spec_page_size() != 0,
        },
        (result is Ok) <==> (spec_addr(&addr) % crate::hal::mem::spec_page_size() == 0),
;

} // verus!
