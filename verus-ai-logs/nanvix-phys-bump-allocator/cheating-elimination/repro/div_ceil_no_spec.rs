// Minimal reproducer for the `align_up` exec rewrite in
// `src/libs/bump_allocator/src/lib.rs`.
//
// This is the ORIGINAL `align_up` body, which uses the `usize::div_ceil`
// intrinsic. Verus ships no specification for `div_ceil`, so the function
// cannot be verified as written — which is why the real code open-codes the
// ceiling division (see `lemma_ceil_div` in `lib.proof.rs`).
//
// Reproduce with:
//   PATH="<verus-dir>:$PATH" verus div_ceil_no_spec.rs
//
// Captured Verus error (verus 0.2026.05.24):
//
//   error: `core::num::impl&%11::div_ceil` is not supported (note: you may be
//   able to add a Verus specification to this function with
//   `assume_specification`) (note: the vstd library provides some specification
//   for the Rust std library, but it is currently limited)
//     --> div_ceil_no_spec.rs:NN:5
//      |
//      |     value.div_ceil(alignment).checked_mul(alignment)
//      |     ^^^^^^^^^^^^^^^^^^^^^^^^^
//      |
//      = help: The following declaration may resolve this error:
//              pub assume_specification [core::num::<impl usize>::div_ceil] (_0: usize, _1: usize) -> usize;
//
//   error: aborting due to 1 previous error
//
// The suggested `assume_specification` is an unapproved external-bottom trust
// assumption (`div_ceil` is not in `verus-ai-logs/tcb-allowed.md`), so it is
// NOT an acceptable fix here. The semantically-equivalent open-coded rewrite is
// used instead.

use vstd::prelude::*;

verus! {

pub const fn align_up(value: usize, alignment: usize) -> (result: Option<usize>)
    ensures
        true,
{
    if alignment == 0 {
        return None;
    }
    value.div_ceil(alignment).checked_mul(alignment)
}

fn main() {}

} // verus!
