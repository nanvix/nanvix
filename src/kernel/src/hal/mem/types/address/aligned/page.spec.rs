use crate::hal::mem::spec_page_size;

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

// Dependency contract for the `PageAligned` `Deref` method.
//
// The `impl<T: Address> Address for PageAligned<T>` block is now `#[verus_verify]`, so its
// `into_raw_value` is verified in-body against the `Address` trait's `#[verus_spec]` contract
// and no longer needs a trusted specification here.
//
// `Deref`, however, is a `core` (std) trait with no Verus contract to inherit, so the
// `impl<T: Address> Deref for PageAligned<T>` block stays a plain impl and its `deref` is
// treated as *external*. The HAL frame layer and `mm::phys` call it from verified code, so it
// needs a trusted specification. This was previously declared in `mm/phys/frame.spec.rs`; it
// belongs here next to the type it describes. Removing it (the contract equals `result == a@`)
// breaks the verus build with "cannot use function ... which is ignored because it is ...
// external".
pub assume_specification<T: Address>[ <crate::hal::mem::PageAligned<T> as ::core::ops::Deref>::deref ](
    a: &crate::hal::mem::PageAligned<T>,
) -> (result: &<crate::hal::mem::PageAligned<T> as ::core::ops::Deref>::Target)
    ensures
        (*result)@ == a@,
;

impl<T: Address> PageAligned<T>
{
    pub open spec fn inv(&self) -> bool
    {
        self@ % spec_page_size() == 0
    }
}

} // verus!
