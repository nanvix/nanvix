// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

///
/// # Description
///
/// Logs an INFO-level formatted message.
///
/// # Parameters
///
/// - `$($arg:tt)*`: Formatted message to be logged.
///
macro_rules! info{
	( $($arg:tt)* ) => ({
		use core::fmt::Write;
		use crate::klog::KlogLevel;
		let _ = write!(&mut crate::klog::Klog::get(module_path!(), KlogLevel::Info), $($arg)*);
	})
}

///
/// # Description
///
/// Logs a TRACE-level formatted message.
///
/// # Parameters
///
/// - `$($arg:tt)*`: Formatted message to be logged.
///
macro_rules! trace{
	( $($arg:tt)* ) => ({
		use core::fmt::Write;
		use crate::klog::KlogLevel;
		let _ = write!(&mut crate::klog::Klog::get(module_path!(), KlogLevel::Trace), $($arg)*);
	})
}

///
/// # Description
///
/// Logs a WARN-level formatted message.
///
/// # Parameters
///
/// - `$($arg:tt)*`: Formatted message to be logged.
///
macro_rules! warn{
	( $($arg:tt)* ) => ({
		use core::fmt::Write;
		use crate::klog::KlogLevel;
		let _ = write!(&mut crate::klog::Klog::get(module_path!(), KlogLevel::Warn), $($arg)*);
	})
}

///
/// # Description
///
/// Logs an ERROR-level formatted message.
///
/// # Parameters
///
/// - `$($arg:tt)*`: Formatted message to be logged.
///
macro_rules! error{
	( $($arg:tt)* ) => ({
		use core::fmt::Write;
		use crate::klog::KlogLevel;
		let _ = write!(&mut crate::klog::Klog::get(module_path!(), KlogLevel::Error), $($arg)*);
	})
}

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
macro_rules! static_assert_alignment {
    ($struct_type:ty, $alignment:expr) => {
        const _: () = [(); 1][(core::mem::align_of::<$struct_type>() == $alignment) as usize ^ 1];
    };
}

macro_rules! run_test {
    ($test_func:ident) => {{
        let result: bool = $test_func();
        #[cfg(test)]
        info!("{}: {}", if result { "passed" } else { "FAILED" }, stringify!($test_func));
        assert!(result);
        result
    }};
}
