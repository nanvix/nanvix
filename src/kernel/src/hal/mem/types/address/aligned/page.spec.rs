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

} // verus!
