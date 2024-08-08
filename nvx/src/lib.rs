// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![feature(panic_info_message)]
#![no_std]

//==================================================================================================
// Modules
//==================================================================================================

pub mod logging;
mod panic;

//==================================================================================================
// Exports
//==================================================================================================

/// Architecture-specific symbols.
pub use ::kcall::arch;

/// Debug facilities.
pub mod debug;

/// Event handling kernel calls.
pub mod event;

/// Inter-Process Communication (IPC) kernel calls.
pub mod ipc;

/// System configuration.
pub use ::kcall::sys;

/// Memory management kernel calls.
pub mod mm;

/// Process management kernel calls.
pub mod pm;

#[macro_export]
macro_rules! log{
	( $($arg:tt)* ) => ({
		use core::fmt::Write;
		use $crate::logging::Logger;
		let _ = writeln!(&mut Logger, $($arg)*);
	})
}
