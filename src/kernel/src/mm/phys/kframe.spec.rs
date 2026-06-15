verus! {

/// Abstract view of a kernel frame: the physical address of the owned frame.
impl View for KernelFrame {
    type V = int;

    closed spec fn view(&self) -> int {
        self.base@
    }
}

impl KernelFrame {
    /// Well-formedness of a kernel-frame handle: its owned physical address is page aligned.
    ///
    /// Purely structural — it makes **no** ownership claim (it does not assert that the frame
    /// is allocated). Page alignment is the one caller-visible fact `base()`'s consumers rely on:
    /// `into_page_address` and the `KernelStack` index arithmetic behind it assume the returned
    /// `FrameAddress` is page aligned. Stated entirely on the abstract address `self@`, so it
    /// leaks no implementation detail. Mirror of `UserFrame::inv`.
    pub open spec fn inv(&self) -> bool {
        self@ % crate::hal::mem::spec_page_size() == 0
    }
}

} // verus!
