use vstd::prelude::*;

// Minimized legal Rust: a plain global `static mut`, annotated the way the probe
// injects it. Verus rejects `static mut` in its frontend.
#[verus_verify]
static mut COUNTER: u32 = 0;

fn main() {}
