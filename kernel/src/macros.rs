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

macro_rules! run_test {
    ($test_func:ident) => {{
        let result: bool = $test_func();
        #[cfg(test)]
        info!("{}: {}", if result { "passed" } else { "FAILED" }, stringify!($test_func));
        assert!(result);
        result
    }};
}
