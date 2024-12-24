// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "smp")]
use crate::pm::sync::spinlock::Spinlock;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Lock for the standard output.
#[cfg(feature = "smp")]
pub static STDOUT_LOCK: Spinlock = Spinlock::new();

//==================================================================================================
// Macros
//==================================================================================================

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
		#[cfg(feature = "smp")]
		use crate::macros::STDOUT_LOCK;
		use ::core::fmt::Write;
		if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Info {
			#[cfg(feature = "smp")]
			let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
			let _ = write!(
				&mut crate::klog::Klog::get(module_path!(), crate::klog::KlogLevel::Info),
				$($arg)*
			);
		}
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
		#[cfg(feature = "smp")]
		use crate::macros::STDOUT_LOCK;
		use ::core::fmt::Write;
		if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Trace {
			#[cfg(feature = "smp")]
			let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
			let _ = write!(
				&mut crate::klog::Klog::get(module_path!(), crate::klog::KlogLevel::Trace),
				$($arg)*
			);
		}
	})
}

///
/// # Description
///
/// Logs a DEBUG-level formatted message.
///
/// # Parameters
///
/// - `$($arg:tt)*`: Formatted message to be logged.
///
macro_rules! debug{
	( $($arg:tt)* ) => ({
		#[cfg(feature = "smp")]
		use crate::macros::STDOUT_LOCK;
		use ::core::fmt::Write;
		if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Debug {
			#[cfg(feature = "smp")]
			let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
			let _ = write!(
				&mut crate::klog::Klog::get(module_path!(), crate::klog::KlogLevel::Debug),
				$($arg)*
			);
		}
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
		#[cfg(feature = "smp")]
		use crate::macros::STDOUT_LOCK;
		use ::core::fmt::Write;
		if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Warn {
			#[cfg(feature = "smp")]
			let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
			let _ = write!(
				&mut crate::klog::Klog::get(module_path!(), crate::klog::KlogLevel::Warn),
				$($arg)*
			);
		}
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
		#[cfg(feature = "smp")]
		use crate::macros::STDOUT_LOCK;
		use ::core::fmt::Write;
		if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Error {
			#[cfg(feature = "smp")]
			let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
			let _ = write!(
				&mut crate::klog::Klog::get(module_path!(), crate::klog::KlogLevel::Error),
				$($arg)*
			);
		}
	})
}

#[cfg(test)]
macro_rules! run_test {
    ($test_func:ident) => {{
        let result: bool = $test_func();
        #[cfg(test)]
        info!("{}: {}", if result { "passed" } else { "FAILED" }, stringify!($test_func));
        assert!(result);
        result
    }};
}
