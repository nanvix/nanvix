verus! {

// `PAGE_ALIGNMENT` is an external (arch-crate) constant. Model it so that verified
// exec code referencing it has a Verus specification. The page alignment's numeric
// value is the page size (both are 4096 on the supported target), which is the link
// that lets `from_address` relate `is_aligned(PAGE_ALIGNMENT)` to `spec_page_size()`.
pub assume_specification[ ::arch::mem::PAGE_ALIGNMENT ] -> (result: Alignment)
    ensures
        ::sys::mm::spec_align_value(result) == spec_page_size(),
;

// Success condition for the validating constructor, stated purely on the abstract
// address value. `from_address` validates, it does NOT normalize: success requires the
// *input* to already be page-aligned.
pub open spec fn spec_aligned(addr_view: int) -> bool {
    addr_view % spec_page_size() == 0
}

// Dependency contracts for the `PageAligned` `Address`/`Deref` methods.
//
// The `impl<T: Address> Address for PageAligned<T>` and `impl<T: Address> Deref for
// PageAligned<T>` blocks below are plain (non-`#[verus_verify]`) impls, so Verus treats
// their methods as *external* and cannot use the trait-level `#[verus_spec]` contract. The
// HAL frame layer (`FrameAddress::into_raw_value`/`into_frame_number`) and `mm::phys` call
// these methods from verified code, so they need trusted specifications. These were
// previously declared in `mm/phys/frame.spec.rs`; they belong here next to the type they
// describe. Removing them entirely (the contracts equal `result == a@`) breaks the verus
// build with "cannot use function ... which is ignored because it is ... external".
pub assume_specification<T: Address>[ <crate::hal::mem::PageAligned<T> as ::core::ops::Deref>::deref ](
    a: &crate::hal::mem::PageAligned<T>,
) -> (result: &<crate::hal::mem::PageAligned<T> as ::core::ops::Deref>::Target)
    ensures
        (*result)@ == a@,
;

} // verus!
