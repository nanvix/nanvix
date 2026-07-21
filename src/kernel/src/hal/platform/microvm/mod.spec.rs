verus! {

// The MicroVM platform's guest-virtual -> guest-physical translation map.
//
// MicroVM runs the guest identity-mapped, so the abstract map is the identity
// over the entire usize address space. This is a platform-level invariant (the
// VMM maps GVA == GPA), NOT an implementation artifact: any reimplementation of
// the translation on MicroVM must still yield the identity, so the equality is
// part of the contract, hence `open`.
//
// Total: defined for every address. Deterministic: a function of `gva` alone.
// Frame correspondence (the caller's load-bearing property) and injectivity are
// immediate corollaries of this identity.
pub open spec fn spec_gva_to_gpa(gva: int) -> int {
    gva
}

} // verus!
