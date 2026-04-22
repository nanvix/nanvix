use vstd::prelude::*;

verus! {
struct Guard<'a> {
    value: &'a mut u64,
}
}

fn main() {}
