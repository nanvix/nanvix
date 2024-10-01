// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![cfg_attr(feature = "allocator", feature(allocator_api))]
#![no_std]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(target_os = "none")]
pub mod logging;
#[cfg(target_os = "none")]
mod panic;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "allocator")]
extern crate alloc;

//==================================================================================================
// Exports
//==================================================================================================

/// Architecture-specific symbols.
#[cfg(target_os = "none")]
pub use ::sys::kcall::arch;

/// Debug facilities.
#[cfg(target_os = "none")]
pub mod debug;

/// Event handling kernel calls.
pub mod event;

/// Inter-Process Communication (IPC) kernel calls.
pub mod ipc;

/// System configuration.
pub use ::sys;

/// Memory management kernel calls.
pub mod mm;

/// Process management kernel calls.
pub mod pm;

#[macro_export]
#[cfg(target_os = "none")]
macro_rules! log{
	( $($arg:tt)* ) => ({
		use core::fmt::Write;
		use $crate::logging::Logger;
		let _ = writeln!(&mut Logger, $($arg)*);
	})
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
#[cfg(target_os = "none")]
pub extern "C" fn _start() -> ! {
    extern "Rust" {
        fn main() -> Result<(), ::sys::error::Error>;
    }

    // Initializes the system runtime.
    init();

    // Runs the main function.
    let status: i32 = match unsafe { main() } {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    };

    // Cleans up the system runtime.
    cleanup();

    // Exits the runtime.
    let Err(e) = ::sys::kcall::pm::exit(status);
    panic!("failed to exit process manager daemon: {:?}", e);
}

/// Initializes system runtime.
#[cfg(target_os = "none")]
fn init() {
    if let Err(e) = mm::init() {
        panic!("failed to initialize memory manager: {:?}", e);
    }
}

/// Cleans up system runtime.
#[cfg(target_os = "none")]
fn cleanup() {
    if let Err(e) = mm::cleanup() {
        panic!("failed to cleanup memory manager: {:?}", e);
    }
}
