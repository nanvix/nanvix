use crate::mm::phys::FrameAllocView;

verus! {

impl UserFrame {
    /// Well-formedness of a user-frame handle: its owned physical address is page aligned.
    ///
    /// This is purely structural; ownership is expressed by individual operation contracts.
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
/// The pool carries no spec-readable state of its own; its state is the global frame allocator.
impl View for Upool {
    type V = FrameAllocView;

    uninterp spec fn view(&self) -> FrameAllocView;
}

} // verus!
