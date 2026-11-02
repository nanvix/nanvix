// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    delivery_test,
    kill_test,
    sigframe,
    signal_test,
    test_detach,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs all in-kernel unit tests for the process state module.
pub(in crate::pm::process) fn test() -> bool {
    let mut passed: bool = true;
    passed &= delivery_test::test();
    passed &= test_detach::test();
    passed &= signal_test::test();
    passed &= sigframe::test();
    passed &= kill_test::test();
    passed
}
