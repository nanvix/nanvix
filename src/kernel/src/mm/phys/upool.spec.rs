verus! {

impl UserFrame {
    /// Well-formedness of a user-frame handle: its owned physical address is page aligned.
    ///
    /// This is purely structural; it makes **no** ownership claim (it does not assert the frame
    /// is allocated or has a positive refcount). Ownership is a per-operation transition fact,
    /// not a handle invariant, because `new` legitimately fabricates handles from
    /// PTE-recovered addresses (the probe / take-to-free / re-wrap idioms) without touching the
    /// frame partition. Surfaced so the refcount-affecting methods can discharge the frame
    /// layer's `frame.inv()` precondition.
    pub open spec fn inv(&self) -> bool {
        self@ % crate::hal::mem::spec_page_size() == 0
    }
}

} // verus!
