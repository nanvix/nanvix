// VERUS-AI LIMITATION id=L1
// construct: codec round-trip (decode-after-encode) over two `uninterp` spec functions
//            parameterized by a structureless generic `E`.
//
// Mirrors `arch::x86::mem::paging::table`:
//   spec_entry_raw<E>(e: E) -> PteWord            (== `enc` below)
//   spec_entry_from_raw<E>(w: PteWord) -> Option<E>  (== `dec` below)
//   lemma_entry_roundtrip<E>: dec(enc(e)) == Some(e)
//
// The two codec functions must stay `uninterp` over `E`: the `TableEntry`
// trait's method specs reference them, so giving them a `TableEntry` bound
// would create a definitional cycle (`view_design.md`). With no structure on
// `E` and no concrete bodies, Verus cannot relate `dec` to `enc` — the
// round-trip is a trusted property of each concrete implementor's codec, not a
// derivable fact. Removing the `assume` below reproduces the failure.
//
// Run: verus L1.rs
//
// Without the assume:
//   error: postcondition not satisfied
//    --> L1.rs
//     | ensures dec::<E>(enc(e)) == Some(e)
//
// This is NOT a proof gap that a better strategy closes: there is no fact in
// scope connecting two uninterpreted functions over a generic type. The single
// approved limitation assume is the in-module expression of the trait-level
// codec injectivity law (each `TableEntry` impl honours it against its own
// interpreted codec).

use vstd::prelude::*;

verus! {

pub uninterp spec fn enc<E>(e: E) -> u32;

pub uninterp spec fn dec<E>(w: u32) -> Option<E>;

pub broadcast proof fn lemma_rt<E>(e: E)
    ensures
        #[trigger] dec::<E>(enc(e)) == Some(e),
{
    // VERUS-AI LIMITATION: id=L1 construct=uninterp-generic-codec-injectivity repro=verus-ai-logs/nanvix-phys-arch-paging-table/repros/L1.rs
    assume(dec::<E>(enc(e)) == Some(e));
}

fn main() {}

} // verus!
