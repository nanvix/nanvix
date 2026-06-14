verus! {

impl Inner {
    pub open spec fn inv(&self) -> bool
    {
        &&& self@.wf()
        &&& self.internal_inv()
    }
}

// =================================================================================================
// Dependency contracts for not-yet-verified modules.
//
// The functions below live in the kernel HAL address/region layer, which is not verified yet. They
// are given trusted external specifications here so that the frame-allocator bodies can be
// translated by Verus. These declarations are placeholders: when the underlying modules are
// verified, their real specifications will supersede these and the declarations below will be
// removed.
//
// Note: `FrameNumber` (the `arch` crate) now carries its own verified `#[verus_spec]` contracts
// (`View` + `FrameNumber::spec_max()`), so its placeholder `external_type_specification`
// (`ExFrameNumber`) was removed — the real datatype specification supersedes it.
// =================================================================================================

// `::arch::mem::FRAME_SIZE` now carries its own verified spec in the `arch` crate (it equals
// `PAGE_SIZE`), so the placeholder `assume_specification[FRAME_SIZE]` is superseded and removed.
// Callers that relied on `FRAME_SIZE == spec_page_size()` still get it: `spec_page_size()` is now
// defined as `::arch::mem::PAGE_SIZE as int` and `FRAME_SIZE == PAGE_SIZE` holds by the verified
// constants.
//
// Note: the placeholder `assume_specification`s for `<PageAligned<T> as Address>::into_raw_value`
// and `<PageAligned<T> as Deref>::deref` were removed. `into_raw_value` is covered by the
// verified `Address::into_raw_value` trait contract (`#[verus_spec(ensures result as int ==
// self@)]` in `sys::mm::address`), which applies at every call site, so the workspace-internal
// placeholder was redundant. `deref` is not used anywhere in `mm::phys`, so its placeholder was
// dead and is dropped as well.

}
