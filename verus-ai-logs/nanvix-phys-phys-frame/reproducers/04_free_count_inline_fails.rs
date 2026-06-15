// Minimal reproducer for the `free_count` exec rewrite (frame.rs).
//
// Demonstrates WHY the original single-expression idiom
//     inner.bitmap.number_of_bits() - inner.bitmap.usage()
// must be split into named `let nbits` / `let used` bindings.
//
// Models the EXACT architecture:
//   * a `Bitmap` whose `inv()` hides the backing-slice length, so the abstract
//     `num_bits` bound is OPAQUE to any caller (== ::bitmap::Bitmap).
//   * `number_of_bits()` whose exec postcondition materializes `result as int ==
//     num_bits` and `result > 0` — the ONLY place `num_bits >= 0` becomes known.
//   * a `lemma_free_count` that REQUIRES `bitmap@.num_bits >= 0` (== frame.proof.rs).
//   * `free_count`, which calls the lemma then returns `number_of_bits() - usage()`.
//
// PASS form  (split bindings): `let nbits = number_of_bits();` surfaces
//            `nbits >= 0` ⇒ `num_bits >= 0`, discharging the lemma precondition.
// FAIL form  (inlined expr): the lemma runs before any binding materializes the
//            `usize` fact ⇒
//            `error: precondition not satisfied ... bitmap@.num_bits >= 0`.
//
// Observed FAIL (inlined) against the real tree
// (`make verify-kernel MODULE=mm::phys`):
//
//   error: precondition not satisfied
//      --> src/kernel/src/mm/phys/frame.rs:851:9
//   851 |         lemma_free_count(inner);
//    95 |         inner.bitmap@.num_bits >= 0,
//       |         --------------------------- failed precondition
//   verification results:: 30 verified, 1 errors
use vstd::prelude::*;

verus! {

pub struct BitmapView {
    pub num_bits: int,
    pub usage: int,
}

pub struct Bitmap {
    pub n: u32,
    pub u: u32,
}

impl View for Bitmap {
    type V = BitmapView;
    closed spec fn view(&self) -> BitmapView {
        BitmapView { num_bits: self.n as int, usage: self.u as int }
    }
}

impl Bitmap {
    // `inv()` references the private fields but, like ::bitmap::Bitmap::inv(),
    // exposes NO usable lower bound on `num_bits` to an external caller.
    pub closed spec fn inv(&self) -> bool {
        self.u <= self.n
    }

    // == ::bitmap number_of_bits(): the `usize`/`u32` result postcondition is the
    // ONLY materialization point of `num_bits >= 0` for a caller.
    pub fn number_of_bits(&self) -> (result: u32)
        requires self.inv(),
        ensures
            result as int == self@.num_bits,
            result > 0,
    {
        self.n
    }

    pub fn usage(&self) -> (result: u32)
        requires self.inv(),
        ensures result as int == self@.usage,
    {
        self.u
    }
}

// == frame.proof.rs lemma_free_count: REQUIRES the opaque `num_bits >= 0`.
pub proof fn lemma_free_count(b: &Bitmap)
    requires
        b.inv(),
        b@.num_bits >= 0,
{
}

// ---- PASS form: split bindings materialize `num_bits >= 0` ----
pub fn free_count_ok(b: &Bitmap) -> (result: u32)
    requires b.inv(),
{
    let nbits: u32 = b.number_of_bits();
    let used: u32 = b.usage();
    proof! { lemma_free_count(b); }
    nbits - used
}

// ---- FAIL form (uncomment to reproduce): inlined expression ----
// pub fn free_count_inline(b: &Bitmap) -> (result: u32)
//     requires b.inv(),
// {
//     proof! { lemma_free_count(b); }     // error: precondition not satisfied:
//                                         //        b@.num_bits >= 0
//     b.number_of_bits() - b.usage()
// }

} // verus!

fn main() {}
