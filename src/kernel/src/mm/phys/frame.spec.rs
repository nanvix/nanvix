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

pub assume_specification[ ::arch::mem::FRAME_SIZE ] -> (result: usize)
    ensures
        result == crate::hal::mem::spec_page_size(),
;

pub assume_specification<T: ::sys::mm::Address>[ <crate::hal::mem::PageAligned<T> as ::sys::mm::Address>::into_raw_value ](
    a: crate::hal::mem::PageAligned<T>,
) -> (result: usize)
    ensures
        result as int == a@,
;

pub assume_specification<T: ::sys::mm::Address>[ <crate::hal::mem::PageAligned<T> as ::core::ops::Deref>::deref ](
    a: &crate::hal::mem::PageAligned<T>,
) -> (result: &<crate::hal::mem::PageAligned<T> as ::core::ops::Deref>::Target)
    ensures
        (*result)@ == a@,
;

}
