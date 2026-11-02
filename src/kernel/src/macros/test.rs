// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Macros
//==================================================================================================

///
/// # Description
///
/// Runs an in-kernel test function, logs its outcome, and asserts that it passed.
///
/// # Parameters
///
/// - `$test_func`: Identifier of a function that takes no arguments and returns a `bool`.
///
/// # Returns
///
/// The result of the test function.
///
macro_rules! run_test {
    ($test_func:ident) => {{
        let result: bool = $test_func();
        info!("{}: {}", if result { "passed" } else { "FAILED" }, stringify!($test_func));
        assert!(result);
        result
    }};
}
