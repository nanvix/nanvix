// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    rand,
    srand,
};
use crate::RAND_MAX;

/// Collects `N` consecutive values from `rand()`.
fn sequence<const N: usize>() -> [i32; N] {
    ::core::array::from_fn(|_| unsafe { rand() })
}

// All assertions share a single test because `rand()`/`srand()` operate on a process-global
// seed. Keeping them in one test serializes access and lets the implicit-seed case observe the
// pristine initial state before any call to `srand()`.
#[test]
fn rand_behaviour() {
    // Calling `rand()` before any call to `srand()` must yield the same sequence as `srand(1)`.
    let implicit: [i32; 8] = sequence();
    unsafe { srand(1) };
    let seeded_one: [i32; 8] = sequence();
    assert_eq!(implicit, seeded_one);

    // The selected LCG matches musl's sequence.
    assert_eq!(
        seeded_one,
        [
            0,
            740_882_966,
            1_616_430_695,
            1_708_849_955,
            1_669_437_588,
            406_334_850,
            276_737_754,
            1_296_416_700,
        ]
    );

    // Re-seeding with the same value repeats the sequence.
    unsafe { srand(1) };
    let first: [i32; 8] = sequence();
    unsafe { srand(1) };
    let second: [i32; 8] = sequence();
    assert_eq!(first, second);

    // Generated values stay within `[0, RAND_MAX]`.
    unsafe { srand(42) };
    for _ in 0..100 {
        let val = unsafe { rand() };
        assert!((0..=RAND_MAX).contains(&val));
    }
}
