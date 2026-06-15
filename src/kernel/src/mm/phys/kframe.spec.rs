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

// Trusted exec-boundary contract for the [`KernelFrame::map_frame`] helper. `map_frame` performs
// the identity-mapping side effect by calling `mm::virt::identity_map_page`, whose precondition
// `identity_map_view().inv()` is a global invariant of the not-yet-verified `mm::virt` module that
// cannot be discharged from within `mm::phys`. The side effect is observable to no `mm::phys`
// caller (it only installs a kernel page-table entry), so the trusted contract is empty: no
// `requires` (any aligned/owned frame is accepted) and no abstract `ensures` (it returns a plain
// `Result<(), Error>`). This trusts strictly less than the previous `external_body` on `new`: the
// owned-frame identity and well-formedness postconditions of `new` are now machine-verified; only
// the cross-module page-table side effect remains trusted, exactly at the `mm::virt` boundary.
pub assume_specification[ KernelFrame::map_frame ](base: FrameAddress) -> Result<(), Error>;

} // verus!
