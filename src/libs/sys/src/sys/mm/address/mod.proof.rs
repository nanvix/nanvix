// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// `Address` trait — Proofs
//
// The in-scope items are trait *method declarations* (`is_aligned`,
// `into_raw_value`, `from_raw_value`): they have no bodies, so they carry no
// proof obligations of their own — the obligations fall on the implementors,
// which are verified (or trusted) in their own modules. The shared spec
// vocabulary (`spec_addr`, `align_value`, `addr_is_aligned`, `addr_inv`) lives
// in `mod.spec.rs`. No standalone lemmas are required here.

verus! { } // verus!
