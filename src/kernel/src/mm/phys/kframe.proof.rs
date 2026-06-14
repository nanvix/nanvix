// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// KernelFrame - Proofs
//
// No proof lemmas are required for `new` / `base` / `drop`: the address-identity
// of `new` follows directly from the `View` impl, `base` is a trivial accessor,
// and the `Drop` invariant-preservation guarantee is provided verbatim by the
// `frame::free` shim contract.

verus! { } // verus!
