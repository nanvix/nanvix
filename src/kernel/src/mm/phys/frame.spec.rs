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

pub assume_specification[ ::arch::mem::paging::FrameNumber::from_raw_value ](_v: usize)
    -> Option<::arch::mem::paging::FrameNumber>;

pub assume_specification[ ::arch::mem::paging::FrameNumber::into_raw_value ](_f: ::arch::mem::paging::FrameNumber)
    -> usize;

pub assume_specification[ crate::hal::mem::FrameAddress::from_frame_number ](
    _f: ::arch::mem::paging::FrameNumber,
) -> Result<crate::hal::mem::FrameAddress, ::sys::error::Error>;

pub assume_specification[ crate::hal::mem::FrameAddress::into_frame_number ](
    _a: crate::hal::mem::FrameAddress,
) -> ::arch::mem::paging::FrameNumber;

pub assume_specification[ crate::hal::mem::PhysicalAddress::into_frame_number ](
    _a: crate::hal::mem::PhysicalAddress,
) -> ::arch::mem::paging::FrameNumber;

pub assume_specification<T: ::sys::mm::Address>[ crate::hal::mem::TruncatedMemoryRegion::<T>::start ](
    _r: &crate::hal::mem::TruncatedMemoryRegion<T>,
) -> crate::hal::mem::PageAligned<T>;

pub assume_specification<T: ::sys::mm::Address>[ crate::hal::mem::TruncatedMemoryRegion::<T>::size ](
    _r: &crate::hal::mem::TruncatedMemoryRegion<T>,
) -> usize;

pub assume_specification<T: ::sys::mm::Address>[ <crate::hal::mem::PageAligned<T> as ::sys::mm::Address>::into_raw_value ](
    _a: crate::hal::mem::PageAligned<T>,
) -> usize;

pub assume_specification<T: ::sys::mm::Address>[ <crate::hal::mem::PageAligned<T> as ::core::ops::Deref>::deref ](
    _a: &crate::hal::mem::PageAligned<T>,
) -> &<crate::hal::mem::PageAligned<T> as ::core::ops::Deref>::Target;

}
