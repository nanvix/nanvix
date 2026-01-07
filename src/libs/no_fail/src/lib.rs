// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Macros
//==================================================================================================

///
/// # Description
///
/// Executes the provided block, ensures it evaluates to `Result<$T, Infallible>`, and returns the
/// contained value, turning any fallible construction into a compile-time error.
///
/// # Parameters
///
/// - `$T`: Type expected from the fallible-free computation.
/// - `$body`: Block that must evaluate to `Result<$T, Infallible>`.
///
/// # Returns
///
/// Returns the inner `$T` value produced by the block.
///
#[macro_export]
macro_rules! no_fail {
    ($T:ty, $body:block) => {{
        let result: ::core::result::Result<$T, ::core::convert::Infallible> = { $body };
        match result {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(err) => match err {},
        }
    }};
}
