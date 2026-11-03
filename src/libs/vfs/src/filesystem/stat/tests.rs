// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests Stat equality and debug.
#[test]
fn stat_clone_eq_debug() {
    let stat: Stat = Stat::new(42, false, 0, 0, 0);
    let cloned: Stat = stat;
    assert_eq!(stat, cloned, "clone should preserve equality");

    let other: Stat = Stat::new(0, true, 0, 0, 0);
    assert_ne!(stat, other, "different stats should not be equal");

    assert_eq!(stat.size(), 42, "size accessor should return 42");
    assert!(!stat.is_dir(), "is_dir accessor should return false");

    let debug: alloc::string::String = alloc::format!("{stat:?}");
    assert!(debug.contains("42"), "debug output should contain file size");
}
