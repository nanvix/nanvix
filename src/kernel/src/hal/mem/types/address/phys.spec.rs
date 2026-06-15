use crate::hal::mem::spec_page_size;
use vstd::arithmetic::power2::pow2;

verus! {

// =================================================================================================
// Abstract models of frame-number quantities, realized by the verified `arch` crate.
//
// `FrameNumber` is a newtype defined in the `arch` crate. Its real `#[verus_spec]` contracts (a
// `View` giving the integer index `frame@`, and the associated bound `FrameNumber::spec_max()`)
// supersede the former placeholder `assume_specification`s. These helpers re-expose those arch
// quantities under the names the kernel proofs already use.
// =================================================================================================

// The integer index (raw value) of a frame number: its `arch` view.
pub open spec fn spec_frame_raw_value(frame: FrameNumber) -> int {
    frame@
}

// The largest representable frame index (the `arch` bound `FrameNumber::spec_max()`).
pub open spec fn spec_max_frame_number() -> int {
    FrameNumber::spec_max() as int
}

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
// `VirtualAddress` (the `sys` crate) and `FRAME_SHIFT` (the `arch` crate) are not yet
// verified. They are given trusted external specifications so that the verified `PhysicalAddress`
// bodies can be translated. These declarations are placeholders: when the underlying modules are
// verified, their real specifications will supersede these.
// =================================================================================================

// Note: `VirtualAddress::new` now carries its own verified `#[verus_spec]` in the `sys` crate
// (`result@ == value as int`), so its placeholder `assume_specification` was removed — the real
// specification supersedes it.

// EXPERIMENT: temporarily commented out for review
// pub assume_specification[ <::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value ](
//     addr: VirtualAddress,
// ) -> (result: usize)
//     ensures
//         result as int == addr@,
// ;

// Note: `arch::mem::FRAME_SHIFT` now carries its own verified modeling in the `arch` crate
// (`#[verus_verify]` on the constant, transparently `PAGE_SHIFT`), so its placeholder
// `assume_specification` was removed — the real definition supersedes it. The facts the proofs
// relied on (`FRAME_SHIFT < 32` and `spec_page_size() == pow2(FRAME_SHIFT)`) now follow from the
// transparent constant values together with `vstd`'s `lemma2_to64` (see `into_frame_number`).

// Note: `FrameNumber::from_raw_value` / `into_raw_value` now carry their own verified `#[verus_spec]`
// contracts in the `arch` crate, so their placeholder `assume_specification`s were removed — the
// real specifications supersede them (see `spec_frame_raw_value` / `spec_max_frame_number` above,
// which now re-expose the arch quantities).

} // verus!
