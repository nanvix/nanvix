// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

///
/// # Description
///
/// Asserts if a type has an expected size at compile time.
///
/// # Parameters
///
/// - `$struct_type:ty`: Type to be checked.
/// - `$expected_size:expr`: Expected size of the type.
///
#[macro_export]
macro_rules! static_assert_size {
    ($struct_type:ty, $expected_size:expr) => {
        const _: () =
            [(); 1][(core::mem::size_of::<$struct_type>() == $expected_size) as usize ^ 1];
    };
}

///
/// # Description
///
/// Asserts if a type has an expected alignment at compile time.
///
/// # Parameters
///
/// - `$struct_type:ty`: Type to be checked.
/// - `$alignment:expr`: Expected alignment of the type.
///
#[macro_export]
macro_rules! static_assert_alignment {
    ($struct_type:ty, $alignment:expr) => {
        const _: () = [(); 1][(core::mem::align_of::<$struct_type>() == $alignment) as usize ^ 1];
    };
}
