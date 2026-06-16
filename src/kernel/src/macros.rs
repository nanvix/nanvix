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
/// Helper macro to extract the current function name using nightly Rust features.
/// This creates a closure and extracts the function name from its type signature.
///
/// # Returns
///
/// A `&'static str` containing the function name, or `"<unknown>"` if extraction fails.
///
#[cfg(not(verus_keep_ghost))]
macro_rules! extract_function_name {
    () => {{
        let closure = || {};
        let closure_type_name: &'static str = ::core::any::type_name_of_val(&closure);

        // Parse the function name from the closure type name.
        // Format: "module::function_name::{{closure}}"
        if let Some(start) = closure_type_name.rfind("::") {
            let before_closure: &str = &closure_type_name[..start];
            if let Some(func_start) = before_closure.rfind("::") {
                &before_closure[func_start + 2..]
            } else {
                before_closure
            }
        } else {
            "<unknown>"
        }
    }};
}

///
/// # Description
///
/// Logs an INFO-level formatted message with function context.
///
/// # Parameters
///
/// - `$($arg:tt)*`: Formatted message to be logged.
///
macro_rules! info{
	( $($arg:tt)* ) => ({
		#[cfg(not(verus_keep_ghost))]
		{
			#[cfg(feature = "smp")]
			use crate::macros::STDOUT_LOCK;
			use ::core::fmt::Write;
			if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Info {
				#[cfg(feature = "smp")]
				let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
				let _ = write!(
					&mut crate::klog::Klog::get(
						module_path!(),
						crate::klog::KlogLevel::Info,
						extract_function_name!()
					),
					$($arg)*
				);
			}
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
		#[cfg(not(verus_keep_ghost))]
		{
			#[cfg(feature = "smp")]
			use crate::macros::STDOUT_LOCK;
			use ::core::fmt::Write;
			if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Trace {
				#[cfg(feature = "smp")]
				let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
				let _ = write!(
					&mut crate::klog::Klog::get(
						module_path!(),
						crate::klog::KlogLevel::Trace,
						extract_function_name!()
					),
					$($arg)*
				);
			}
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
		#[cfg(not(verus_keep_ghost))]
		{
			#[cfg(feature = "smp")]
			use crate::macros::STDOUT_LOCK;
			use ::core::fmt::Write;
			if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Debug {
				#[cfg(feature = "smp")]
				let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
				let _ = write!(
					&mut crate::klog::Klog::get(
						module_path!(),
						crate::klog::KlogLevel::Debug,
						extract_function_name!()
					),
					$($arg)*
				);
			}
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
		#[cfg(not(verus_keep_ghost))]
		{
			#[cfg(feature = "smp")]
			use crate::macros::STDOUT_LOCK;
			use ::core::fmt::Write;
			if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Warn {
				#[cfg(feature = "smp")]
				let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
				let _ = write!(
					&mut crate::klog::Klog::get(
						module_path!(),
						crate::klog::KlogLevel::Warn,
						extract_function_name!()
					),
					$($arg)*
				);
			}
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
		#[cfg(not(verus_keep_ghost))]
		{
			#[cfg(feature = "smp")]
			use crate::macros::STDOUT_LOCK;
			use ::core::fmt::Write;
			if crate::klog::MAX_LEVEL >= crate::klog::KlogLevel::Error {
				#[cfg(feature = "smp")]
				let _guard: crate::pm::sync::spinlock::SpinlockGuard = STDOUT_LOCK.lock();
				let _ = write!(
					&mut crate::klog::Klog::get(
						module_path!(),
						crate::klog::KlogLevel::Error,
						extract_function_name!()
					),
					$($arg)*
				);
			}
		}
	})
}

#[cfg(feature = "test")]
macro_rules! run_test {
    ($test_func:ident) => {{
        let result: bool = $test_func();
        info!("{}: {}", if result { "passed" } else { "FAILED" }, stringify!($test_func));
        assert!(result);
        result
    }};
}
