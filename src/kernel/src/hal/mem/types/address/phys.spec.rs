// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PhysicalAddress — Specifications
//
// A `PhysicalAddress` abstracts to a single mathematical integer: the raw
// physical address (`PhysicalAddress@ : int`, defined by the `View` impl in
// `phys.rs`, delegating to the inner `VirtualAddress`). The only derived notion
// callers reason about is the *frame* the address lies in
// (`addr / FRAME_SIZE == addr >> FRAME_SHIFT`), used to index the frame
// allocator's bitmaps / refcount arrays.
//
// Trust-boundary helpers pin the not-yet-verified `arch` `FrameNumber` type and
// the `arch`/`sys` library-edge constants/methods this module calls. They mirror
// the existing `sys`/`arch` `assume_specification` boundaries the codebase
// already draws (`::arch::mem::PAGE_SIZE` in `frame.rs`, the `Address` trait
// methods in `page.spec.rs` / `kframe.spec.rs`). `spec_page_size()` is the
// canonical frame size, re-exported from `crate::hal::mem` (defined alongside
// `FrameAddress`); `FRAME_SIZE == PAGE_SIZE == spec_page_size()`.

verus! {

use crate::hal::mem::spec_page_size;
use vstd::arithmetic::power2::pow2;
use vstd::arithmetic::div_mod::{
    lemma_fundamental_div_mod,
    lemma_mod_division_less_than_divisor,
    lemma_div_by_multiple,
    lemma_mod_multiples_basic,
};
use vstd::bits::lemma_usize_shr_is_div;

// `FrameNumber` lives in the `arch` crate, which is not Verus-enabled and has no
// `View`/datatype registration reachable here. Declare it to Verus as an opaque
// external datatype so it may appear in spec-fn parameters and the
// `assume_specification`s below. Its internals are not modeled (`external_body`):
// its abstract index is projected by `spec_frame_raw_value`.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFrameNumber(FrameNumber);

// ── Frame-number trust-boundary projections (arch `FrameNumber`) ──────────────
//
// `FrameNumber` lives in the `arch` crate, which is not Verus-enabled and has no
// `View` impl reachable here (orphan rule). Its abstract index is therefore
// projected by an uninterpreted ghost function, exactly as `view_design.md`
// prescribes. `spec_max_frame_number()` denotes `FrameNumber::MAX`.

// The integer index of a frame number (`0 ..= spec_max_frame_number()`).
pub uninterp spec fn spec_frame_raw_value(frame: FrameNumber) -> int;

// The largest representable frame number. Mirrors `arch FrameNumber::MAX ==
// MAX_ADDRESS / FRAME_SIZE - 1 == usize::MAX / FRAME_SIZE - 1`. Interpreting it
// against `usize::MAX` and `spec_page_size()` is what lets verified constructors
// discharge their `usize` multiply (no-overflow) obligation: a frame index in
// `0 ..= spec_max_frame_number()` scaled by `spec_page_size()` stays `<= usize::MAX`.
pub open spec fn spec_max_frame_number() -> int {
    usize::MAX as int / spec_page_size() - 1
}

// ── Derived spec helpers over the View domain (`int`) ─────────────────────────

// The frame an address lies in: exact integer division, independent of whether
// the implementation shifts (`>> FRAME_SHIFT`) or divides (`/ FRAME_SIZE`).
pub open spec fn spec_frame_number(addr_view: int) -> int {
    addr_view / spec_page_size()
}

// The base (frame-aligned) address of a frame index: `frame * FRAME_SIZE`.
pub open spec fn spec_from_number(frame_view: int) -> int {
    frame_view * spec_page_size()
}

// ── `arch`/`sys` library-edge `assume_specification`s ─────────────────────────

// `::arch::mem::FRAME_SIZE` is the frame size (alias of `PAGE_SIZE`), tied to the
// canonical `spec_page_size()`. It is strictly positive (`== 4096`); positivity is
// the load-bearing fact for the division/modulo reasoning in the constructors.
pub assume_specification[ ::arch::mem::FRAME_SIZE ] -> (result: usize)
    ensures
        result == spec_page_size(),
        spec_page_size() > 0,
;

// `::arch::mem::FRAME_SHIFT == log2(FRAME_SIZE)`. Bounded below the pointer width
// so `raw_addr >> FRAME_SHIFT` is well-defined, and `2^FRAME_SHIFT == FRAME_SIZE`
// so the shift coincides with division by `spec_page_size()`.
pub assume_specification[ ::arch::mem::FRAME_SHIFT ] -> (result: usize)
    ensures
        result < usize::BITS,
        pow2(result as nat) == spec_page_size(),
;

// `VirtualAddress::new` is a pure newtype constructor: the wrapped value is the
// abstract address. (`sys` library edge; the inner module is not yet verified.)
pub assume_specification[ VirtualAddress::new ](value: usize) -> (result: VirtualAddress)
    ensures
        result@ == value as int,
;

// `<VirtualAddress as Address>::into_raw_value` is pure newtype identity: the
// returned raw `usize` equals the abstract address.
pub assume_specification[ <VirtualAddress as Address>::into_raw_value ](
    addr: VirtualAddress,
) -> (result: usize)
    ensures
        result as int == addr@,
;

// `FrameNumber::into_raw_value` projects a frame number to its index; every
// `FrameNumber` value is in range (`0 ..= MAX`) by construction.
pub assume_specification[ FrameNumber::into_raw_value ](frame: FrameNumber) -> (result: usize)
    ensures
        result as int == spec_frame_raw_value(frame),
        0 <= spec_frame_raw_value(frame) <= spec_max_frame_number(),
;

// `FrameNumber::from_raw_value` succeeds iff `value <= MAX`, preserving the index.
pub assume_specification[ FrameNumber::from_raw_value ](value: usize) -> (result: Option<FrameNumber>)
    ensures
        match result {
            Some(f) => value as int <= spec_max_frame_number()
                && spec_frame_raw_value(f) == value as int,
            None => value as int > spec_max_frame_number(),
        },
;

// ── Type invariant ───────────────────────────────────────────────────────────
//
// Every `PhysicalAddress` a verified constructor produces has a *representable
// frame number*: `self@ / FRAME_SIZE <= FrameNumber::MAX`. This is the load-
// bearing fact that makes `into_frame_number` total (its internal
// `FrameNumber::from_raw_value(..).unwrap()` never panics), which the frame
// allocator relies on when using the result directly as a bitmap / refcount
// index. It is *not* alignment (MMIO addresses may be unaligned) and *not*
// RAM-validity (`from_mmio_address` deliberately bypasses that check).
impl PhysicalAddress {
    pub open spec fn inv(&self) -> bool {
        spec_frame_number(self@) <= spec_max_frame_number()
    }
}

// `PhysicalAddress` abstracts to a single mathematical integer: the raw physical
// address, delegated from the inner `VirtualAddress`. Kept here (verification
// material) rather than in `phys.rs` so the exec source carries no cfg-gated
// verification constructs.
impl View for PhysicalAddress {
    type V = int;

    closed spec fn view(&self) -> int {
        self.0@
    }
}

} // verus!
