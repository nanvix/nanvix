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

// `FrameNumber` now lives in the Verus-verified `arch` crate, which natively
// registers it as a datatype and supplies its `View`/contracts. The kernel
// therefore reasons about it directly (no local `external_type_specification`,
// which would now duplicate the arch-native datatype registration). Its abstract
// index is `frame@` (arch's `View for FrameNumber`), projected by
// `spec_frame_raw_value` below.

// ── Frame-number trust-boundary projections (arch `FrameNumber`) ──────────────
//
// `FrameNumber` lives in the `arch` crate, which is not Verus-enabled and has no
// `View` impl reachable here (orphan rule). Its abstract index is therefore
// projected by an uninterpreted ghost function, exactly as `view_design.md`
// prescribes. `spec_max_frame_number()` denotes `FrameNumber::MAX`.

// The integer index of a frame number (`0 ..= spec_max_frame_number()`).
// Defined as arch's native `View` (`frame@`); arch's `FrameNumber::into_raw_value`
// / `from_raw_value` contracts speak in terms of this same abstract index.
pub open spec fn spec_frame_raw_value(frame: FrameNumber) -> int {
    frame@
}

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

// `::arch::mem::FRAME_SIZE` is the frame size (alias of `PAGE_SIZE`). Its native
// `#[verus_verify]` contract in the now-verified `arch` crate supplies its value,
// from which `result == spec_page_size()` and `spec_page_size() > 0` follow. A
// local `assume_specification` here would duplicate the arch-native spec, so it is
// removed.

// `::arch::mem::FRAME_SHIFT == log2(FRAME_SIZE)`. Bounded below the pointer width
// so `raw_addr >> FRAME_SHIFT` is well-defined, and `2^FRAME_SHIFT == FRAME_SIZE`
// so the shift coincides with division by `spec_page_size()`.
pub assume_specification[ ::arch::mem::FRAME_SHIFT ] -> (result: usize)
    ensures
        result < usize::BITS,
        pow2(result as nat) == spec_page_size(),
;

// `VirtualAddress::new` now carries a native `#[verus_spec]` in `sys`
// (`mm::address::virt`), so its contract is imported directly across the crate
// boundary; a local `assume_specification` here would duplicate it (Verus error:
// "duplicate specification for ... ::new"). The guaranteed fact is identical
// (`result@ == value as int`).

// `<VirtualAddress as Address>::into_raw_value` is pure newtype identity: the
// returned raw `usize` equals the abstract address. It remains an
// `assume_specification` trust boundary because this trait-impl method cannot be
// body-verified in `sys` without marking the whole `impl Address for
// VirtualAddress` verified, which pulls the sibling `as_ptr`/`as_mut_ptr` methods
// (unsupported `usize as *const u8` casts) into scope — a Verus front-end
// limitation (see `verus-ai-logs/verus-unsupported.md`).
pub assume_specification[ <VirtualAddress as Address>::into_raw_value ](
    addr: VirtualAddress,
) -> (result: usize)
    ensures
        result as int == addr@,
;

// `FrameNumber::into_raw_value` and `FrameNumber::from_raw_value` now carry native
// `#[verus_spec]` contracts in the verified `arch` crate, imported directly across
// the crate boundary (`result as int == frame@`, i.e. `spec_frame_raw_value(frame)`,
// in range `0 ..= spec_max_frame_number()`; `from_raw_value` succeeds iff
// `value <= spec_max_frame_number()`, preserving the index). Local
// `assume_specification`s here would duplicate them, so they are removed.

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
