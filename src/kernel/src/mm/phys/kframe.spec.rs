verus! {

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

// Dependency contract for the not-yet-verified HAL address layer. `KernelFrame::new`'s body
// constructs the `PageAligned<PhysicalAddress>` it identity-maps via this trait-impl constructor.
// The `Address for PageAligned<T>` impl lives outside `verus!`, so the method is given a trusted
// external specification here so `new`'s body translates. Mirror of the `into_raw_value`/`deref`
// `assume_specification`s in `frame.spec.rs`; it is superseded and removed when the HAL address
// `aligned::page` layer's `Address` impl is verified.
pub assume_specification<T: ::sys::mm::Address>[ <crate::hal::mem::PageAligned<T> as ::sys::mm::Address>::from_raw_value ](
    raw_addr: usize,
) -> (result: Result<crate::hal::mem::PageAligned<T>, ::sys::error::Error>)
    ensures
        match result {
            Ok(r) => r@ == raw_addr as int && r.inv(),
            Err(_) => true,
        },
;

} // verus!
