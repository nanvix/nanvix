verus! {

// Well-formedness invariant for `VirtualAddress`.
//
// `VirtualAddress` is a thin, infallible newtype over a pointer-sized integer.
// Its abstract state (`self@ : int`) is exactly the numeric virtual address.
// The single universal property every constructible value satisfies — and that
// callers performing address arithmetic and round-trips rely on — is that the
// value fits in a `usize` (the pointer-sized address space).
//
// `open` so callers can unfold the bound in arithmetic proofs.
impl VirtualAddress {
    pub open spec fn inv(&self) -> bool {
        0 <= self@ <= usize::MAX as int
    }
}

} // end verus!
