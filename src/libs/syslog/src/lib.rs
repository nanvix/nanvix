// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "std")]
extern crate log;

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(feature = "std")]
mod hosted;
#[cfg(not(feature = "std"))]
mod standalone;

//==================================================================================================
// Macros
//==================================================================================================

///
/// # Description
///
/// Emits a trace-level syslog record tagged with the `SYSCALL` target so kernel
/// instrumentation can be filtered independently. Typically invoked by the
/// `trace_syscall` attribute macro.
///
/// # Parameters
///
/// - `$($arg:tt)*`: Format string and arguments forwarded to `trace!`.
///
#[macro_export]
macro_rules! syscall_trace {
    ($($arg:tt)*) => {
        $crate::trace!(target: ::core::concat!("SYSCALL][", module_path!()), $($arg)*);
    };
}

///
/// # Description
///
/// Emits a trace-level syslog record tagged with the `LIBCALL` target so libc
/// instrumentation can be filtered independently. Typically invoked by the
/// `trace_libcall` attribute macro.
///
/// # Parameters
///
/// - `$($arg:tt)*`: Format string and arguments forwarded to `trace!`.
///
#[macro_export]
macro_rules! libcall_trace {
    ($($arg:tt)*) => {
        $crate::trace!(target: ::core::concat!("LIBCALL][", module_path!()), $($arg)*);
    };
}

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "std")]
pub use hosted::*;
#[cfg(not(feature = "std"))]
pub use standalone::*;

//==================================================================================================
// Re-exports
//==================================================================================================

pub use ::syslog_macros::{
    trace_libcall,
    trace_syscall,
};
