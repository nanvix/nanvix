use crate::mm::phys::FrameAllocView;

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

/// Abstract view of a user frame: the physical address of the owned frame.
impl View for UserFrame {
    type V = int;

    closed spec fn view(&self) -> int {
        self.addr@
    }
}

/// Abstract view of the user page pool: the frame partition it draws from.
///
/// The pool carries no spec-readable state of its own — its real state is the global frame
/// allocator — so its view is uninterpreted. The cross-call transition is realized by the
/// proving-phase ghost token over the singleton allocator; the trust obligation for the two
/// state-affecting operations is tracked by `Upool::new`/`Upool::alloc` being `external_body`.
impl View for Upool {
    type V = FrameAllocView;

    uninterp spec fn view(&self) -> FrameAllocView;
}

} // verus!
