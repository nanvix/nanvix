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
// The functions below live in the `arch` crate and in the kernel HAL
// address/region layer, neither of which is verified yet. They are given
// trusted external specifications here so that the frame-allocator bodies can be
// translated by Verus. These declarations are placeholders: when the underlying
// modules are verified, their real specifications will supersede these and the
// declarations below will be removed.
// =================================================================================================

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFrameNumber(::arch::mem::paging::FrameNumber);

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
