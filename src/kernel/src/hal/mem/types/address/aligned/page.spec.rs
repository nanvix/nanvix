// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PageAligned<T> — Specifications
//
// `PageAligned<T>` is a validated newtype wrapping a memory address (`int`,
// delegated through the inner `T`'s `View`) carrying the static guarantee that
// the address lies on a page boundary. Its abstract value is exactly that
// address (`PageAligned@ : int`, defined by the `View` impl in `page.rs`); the
// page-alignment fact is the property `inv()` over that value.
//
// The two in-scope functions (`into_raw_value`, `from_address`) cannot carry
// their `#[verus_spec]` contracts in place: both must reference the abstract
// address (`@`), which is only defined when `T: View<V = int>`. That bound
// cannot be added to the exec `impl<T: Address> ...` blocks, because
//   * the `View` impls for the address family are `cfg(verus_keep_ghost)`-gated,
//     so `T: View` is unsatisfiable in a normal `cargo build`, and
//   * `region.rs` relies on `PageAligned<T>: Address` for a bare `T: Address`
//     (`TruncatedMemoryRegion<T>(MemoryRegion<PageAligned<T>>)`), which the extra
//     bound would break.
// Additionally `into_raw_value` is a trait method of the external `sys::mm::Address`
// trait, so annotating it in place would force whole-`impl Address` verification.
//
// The contracts are therefore stated here, in the ghost-only spec file, where the
// `T: View<V = int>` bound is available. They are the same caller-facing
// guarantees the sibling `FrameAddress::into_raw_value` (`external_body`) and the
// `kframe.spec.rs` `from_raw_value` shim already draw at this `hal::mem` address
// boundary, and are removed when the address family is verified end-to-end.

verus! {

// `into_raw_value` — pure newtype identity projection of the abstract address.
//
// Callers depend on the returned `usize` equalling the abstract address
// (`result as int == self@`): in-page offset math
// (`a.into_raw_value() - p.into_raw_value()`) and page walking
// (`a.into_raw_value() + k * PAGE_SIZE`) require an identity projection with no
// masking/shifting. A caller holding `self.inv()` further derives
// `result as int % crate::hal::mem::spec_page_size() == 0`, so that fact is implied and not
// restated here.
pub assume_specification<T: Address + View<V = int>> [
    <PageAligned<T> as Address>::into_raw_value
](addr: PageAligned<T>) -> (result: usize)
    ensures
        result as int == addr@,
;

// `from_address` — partial, identity-preserving, validating constructor.
//
// On success the wrapped address is unchanged (`p@ == addr@`) and the page
// alignment invariant is established (`p.inv()`); `from_address` validates, it
// never rounds/normalizes. On failure the input was not page-aligned and there
// is no side effect (value type). The success condition is exactly page
// alignment of the input, stated bidirectionally for liveness.
pub assume_specification<T: Address + View<V = int>> [
    PageAligned::<T>::from_address
](addr: T) -> (result: Result<PageAligned<T>, Error>)
    ensures
        match result {
            Ok(p) => p@ == addr@ && p.inv(),
            Err(_) => addr@ % crate::hal::mem::spec_page_size() != 0,
        },
        (result is Ok) <==> (addr@ % crate::hal::mem::spec_page_size() == 0),
;

} // verus!
