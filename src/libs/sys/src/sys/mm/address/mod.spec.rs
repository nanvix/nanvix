// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// `Address` trait — Specifications
//
// To callers, an `Address` is a single pointer-sized location in an address
// space: one mathematical integer in `[0, usize::MAX]`. The three in-scope
// trait methods form the raw-value boundary of that resource — project the
// abstract address back to a raw `usize` (`into_raw_value`), validate/construct
// one from a raw `usize` (`from_raw_value`), and query its alignment
// (`is_aligned`). See `view_design.md`.
//
// The abstract address is exposed through the universal projection `spec_addr`
// rather than `vstd::View`: `Address` cannot carry a `View<V = int>` supertrait
// because the per-implementor `View` impls (`PhysicalAddress`, `PageAligned`,
// `PageTableAligned`, ...) are `cfg(verus_keep_ghost)`-gated, so the bound would
// be unsatisfiable in a normal `cargo build` and would break the generic
// `impl<T: Address>` blocks (e.g. `region.rs`). This mirrors the kernel's own
// `spec_addr` / `spec_page_size` projections (`hal::mem`), which exist for the
// same reason and which this module is intended to subsume.

verus! {

// Universal abstract-address projection of any `Address` implementor: the
// pointer-sized numeric address it denotes (`int`).
//
// `uninterp` because there is no `View<V = int>` supertrait to define it
// concretely over a bare `T: Address` (see the module note above). It is not a
// free assumption: the trait-method contracts below pin it operationally — a
// value built by `from_raw_value(raw)` projects back through `into_raw_value`
// to exactly `raw`, and `is_aligned` reports `spec_addr` modulo the alignment.
// This is the standard logical-identity pattern, identical to the kernel's
// `hal::mem::spec_addr`; no `external_body` axiom feeds it.
//
// The generic parameter is intentionally *unbounded* (`T`, not `T: Address`).
// Bounding it by `Address` would make this projection part of the `Address`
// trait's own definition cycle (the trait-method contracts below reference
// `spec_addr`, which would in turn reference the trait), which Verus rejects as
// a cyclic self-reference. Callers always instantiate `T = Self` inside an
// `Address` context, so the bound is recovered at every use site.
pub uninterp spec fn spec_addr<T>(addr: &T) -> int;

// The pointer-sized well-formedness bound shared by every `Address`
// implementor. Refinement implementors (aligned / frame-representable) add
// their own predicate on top in their own `inv()`; only the universal bound
// belongs here. Unbounded `T` for the same cycle-avoidance reason as
// `spec_addr` above; instantiated at `T = Self` in every `Address` context.
pub open spec fn addr_inv<T>(addr: &T) -> bool {
    0 <= spec_addr(addr) <= usize::MAX as int
}

// The integer alignment named by an `Alignment` — the positive power of two
// that is its discriminant (`Align4 = 4`, ..., `Align4194304 = 4194304`).
pub open spec fn align_value(a: Alignment) -> int {
    a as int
}

// The single alignment fact `is_aligned` reports and that callers branch on:
// the address is an exact multiple of the alignment.
pub open spec fn addr_is_aligned(addr: int, a: Alignment) -> bool {
    addr % align_value(a) == 0
}

} // verus!
