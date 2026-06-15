// Minimal reproducer for the `free_count` exec rewrite (frame.rs).
//
// Demonstrates WHY the original single-expression idiom
//     inner.bitmap.number_of_bits() - inner.bitmap.usage()
// must be split into named `let nbits` / `let used` bindings.
//
// The key to faithfulness is the MODULE BOUNDARY: the real `::bitmap::Bitmap`
// has a `closed spec fn view` defined in a *different crate*, so
// `inner.bitmap@.num_bits` is OPAQUE in `frame.rs` — its `>= 0` lower bound is
// not visible. Here `Bitmap` lives in an inner `mod bm` and is called from
// outside, so its closed view is likewise hidden (closed-spec bodies are visible
// only within their defining module). A same-module model would leak
// `num_bits == self.n as int >= 0` and FAIL to reproduce the failure.
//
//   * `number_of_bits()` postcondition `result as int == num_bits` is the ONLY
//     place the caller learns `num_bits == nbits (a usize) >= 0`.
//   * `lemma_free_count` REQUIRES `b@.num_bits >= 0` (== frame.proof.rs:95).
//
// PASS form (`free_count_split`): `let nbits = number_of_bits()` materializes the
//   `usize`/`u32` value, so `nbits as int == num_bits` and `nbits >= 0` discharge
//   the lemma precondition. Verifies.
// FAIL form (`free_count_inline`, shipped COMMENTED OUT): the lemma runs before
//   any binding materializes the usize fact, so `num_bits >= 0` is unknown.
//
// Observed output, this reproducer as committed (FAIL form commented out):
//   /mnt/toolchain/verus/verus <this file>
//   verification results:: 4 verified, 0 errors
//
// Observed output, FAIL form uncommented:
//   error: precondition not satisfied
//      --> 04_free_count_inline_fails.rs (free_count_inline)
//         proof! { lemma_free_count(b); }
//      lemma_free_count ... requires b@.num_bits >= 0,
//                                    ---------------- failed precondition
//   verification results:: 4 verified, 1 errors
//
// This matches the real-tree error when the expression is inlined in frame.rs:
//   error: precondition not satisfied
//      --> src/kernel/src/mm/phys/frame.rs:851:9
//   851 |         lemma_free_count(inner);
//    95 |         inner.bitmap@.num_bits >= 0,
//       |         --------------------------- failed precondition
//   verification results:: 30 verified, 1 errors
use vstd::prelude::*;

verus! {

// `Bitmap` lives behind a module boundary so its closed view is hidden from the
// caller — mirroring the real cross-crate `::bitmap` boundary.
mod bm {
    use vstd::prelude::*;

    pub struct BitmapView {
        pub num_bits: int,
        pub usage: int,
    }

    pub struct Bitmap {
        n: u32,
        u: u32,
    }

    impl View for Bitmap {
        type V = BitmapView;
        closed spec fn view(&self) -> BitmapView {
            BitmapView { num_bits: self.n as int, usage: self.u as int }
        }
    }

    impl Bitmap {
        pub closed spec fn inv(&self) -> bool {
            self.u <= self.n
        }

        // == ::bitmap number_of_bits(): the `usize`/`u32` result postcondition is
        // the ONLY materialization point of `num_bits >= 0` for a caller.
        pub fn number_of_bits(&self) -> (result: u32)
            requires self.inv(),
            ensures result as int == self@.num_bits,
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
}

use bm::Bitmap;

// == frame.proof.rs lemma_free_count: REQUIRES the opaque `num_bits >= 0`.
pub proof fn lemma_free_count(b: &Bitmap)
    requires
        b.inv(),
        b@.num_bits >= 0,
{
}

// ---- PASS form: split bindings materialize `num_bits >= 0` ----
pub fn free_count_split(b: &Bitmap) -> (result: u32)
    requires
        b.inv(),
        b@.usage <= b@.num_bits,
{
    let nbits: u32 = b.number_of_bits();
    let used: u32 = b.usage();
    proof! { lemma_free_count(b); }
    nbits - used
}

// ---- FAIL form (uncomment to reproduce): inlined expression ----
// The lemma runs before any binding surfaces the usize fact, so `num_bits >= 0`
// is unknown across the module boundary:
//   error: precondition not satisfied ... b@.num_bits >= 0
//
// pub fn free_count_inline(b: &Bitmap) -> (result: u32)
//     requires
//         b.inv(),
//         b@.usage <= b@.num_bits,
// {
//     proof! { lemma_free_count(b); }
//     b.number_of_bits() - b.usage()
// }

} // verus!

fn main() {}
