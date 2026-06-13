use crate::hal::mem::spec_page_size;
use vstd::arithmetic::power2::pow2;

verus! {

// =================================================================================================
// Abstract models of external (arch-crate) frame-number quantities.
//
// `FrameNumber` is an opaque newtype defined in the (not-yet-verified) `arch` crate, and its
// maximum is the arch constant `FrameNumber::MAX`. Like `spec_page_size()` models the arch
// page-size constant, these uninterpreted functions model a frame number's integer index and the
// largest representable index in the spec domain. They are tied to the concrete exec values by the
// dependency contracts below.
// =================================================================================================

// The integer index (raw value) of a frame number.
pub uninterp spec fn spec_frame_raw_value(frame: FrameNumber) -> int;

// The largest representable frame index (models `arch::mem::paging::FrameNumber::MAX`).
pub uninterp spec fn spec_max_frame_number() -> int;

// =================================================================================================
// Derived spec helpers over the `PhysicalAddress` view (its `int` raw address).
// =================================================================================================

// The frame an address belongs to: `addr / FRAME_SIZE` (equivalently `addr >> FRAME_SHIFT`).
pub open spec fn spec_frame_number(addr_view: int) -> int {
    addr_view / spec_page_size()
}

// The base address of a frame: `frame_index * FRAME_SIZE`.
pub open spec fn spec_from_number(frame_view: int) -> int {
    frame_view * spec_page_size()
}

impl PhysicalAddress {
    // Well-formedness: the address has a representable frame number. This is exactly what makes
    // `into_frame_number` total (its internal `FrameNumber::from_raw_value(..).unwrap()` never
    // panics). `open` so callers' proofs (frame allocator, page tables) can rely on totality.
    pub open spec fn inv(&self) -> bool {
        spec_frame_number(self@) <= spec_max_frame_number()
    }
}

// =================================================================================================
// Dependency contracts for not-yet-verified modules.
//
// `VirtualAddress` (the `sys` crate) and `FrameNumber`/`FRAME_SHIFT` (the `arch` crate) are not yet
// verified. They are given trusted external specifications so that the verified `PhysicalAddress`
// bodies can be translated. These declarations are placeholders: when the underlying modules are
// verified, their real specifications will supersede these.
// =================================================================================================

pub assume_specification[ ::sys::mm::VirtualAddress::new ](value: usize) -> (result: VirtualAddress)
    ensures
        result@ == value as int,
;

pub assume_specification[ <::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value ](
    addr: VirtualAddress,
) -> (result: usize)
    ensures
        result as int == addr@,
;

pub assume_specification[ ::arch::mem::FRAME_SHIFT ] -> (result: usize)
    ensures
        result < 32,
        spec_page_size() == pow2(result as nat),
;

pub assume_specification[ ::arch::mem::paging::FrameNumber::from_raw_value ](value: usize)
    -> (result: Option<FrameNumber>)
    ensures
        value as int <= spec_max_frame_number() ==> (result is Some
            && spec_frame_raw_value(result->Some_0) == value as int),
        value as int > spec_max_frame_number() ==> result is None,
;

pub assume_specification[ ::arch::mem::paging::FrameNumber::into_raw_value ](frame: FrameNumber)
    -> (result: usize)
    ensures
        result as int == spec_frame_raw_value(frame),
        0 <= spec_frame_raw_value(frame) <= spec_max_frame_number(),
;

} // verus!
