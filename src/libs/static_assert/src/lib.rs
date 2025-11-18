// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_extern_crates)]
extern crate compiler_builtins;
#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_extern_crates)]
extern crate core;

//==================================================================================================
// Macros
//==================================================================================================

///
/// # Description
///
/// Asserts if a condition is true at compile time.
///
/// # Parameters
///
/// - `$condition:expr`: Condition to be checked.
///
#[macro_export]
macro_rules! assert_eq {
    ($condition:expr) => {
        const _: () = [(); 1][($condition) as usize ^ 1];
    };
}

///
/// # Description
///
/// Asserts if the size of a type equals an expected size at compile time.
///
/// # Parameters
///
/// - `$typ:ty`: Type to be checked.
/// - `$expected_size:expr`: Expected size of the type.
///
#[macro_export]
macro_rules! assert_eq_size {
    ($typ:ty, $expected_size:expr) => {
        const _: () = [(); 1][(::core::mem::size_of::<$typ>() == $expected_size) as usize ^ 1];
    };
    ($typ:ty, $expected_size:expr) => {
        const _: () = [(); 1][(::core::mem::size_of::<$typ>() == $expected_size) as usize ^ 1];
    };
}

///
/// # Description
///
/// Asserts if the alignment of a type equals an expected alignment at compile time.
///
/// # Parameters
///
/// - `$typ:ty`: Type to be checked.
/// - `$alignment:expr`: Expected alignment of the type.
///
#[macro_export]
macro_rules! assert_eq_align {
    ($typ:ty, $alignment:expr) => {
        const _: () = [(); 1][(::core::mem::align_of::<$typ>() == $alignment) as usize ^ 1];
    };
}
