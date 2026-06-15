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

} // verus!
