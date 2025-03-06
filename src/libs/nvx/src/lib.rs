// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![cfg_attr(feature = "allocator", feature(allocator_api))]
#![cfg_attr(not(feature = "std"), no_std)]

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

/// Execution scheduling kernel calls.
pub mod sched;

#[macro_export]
#[cfg(target_os = "none")]
macro_rules! trace{
    ( $($arg:tt)* ) => ({
		if $crate::logging::MAX_LEVEL >= $crate::logging::LogLevel::Trace {
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::logging::Logger::get(module_path!(), $crate::logging::LogLevel::Trace),
                $($arg)*
            );
        }
    })
}

#[macro_export]
#[cfg(target_os = "none")]
macro_rules! debug{
    ( $($arg:tt)* ) => ({
		if $crate::logging::MAX_LEVEL >= $crate::logging::LogLevel::Debug{
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::logging::Logger::get(module_path!(), $crate::logging::LogLevel::Debug),
                $($arg)*
            );
        }
    })
}

#[macro_export]
#[cfg(target_os = "none")]
macro_rules! info{
    ( $($arg:tt)* ) => ({
		if $crate::logging::MAX_LEVEL >= $crate::logging::LogLevel::Info {
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::logging::Logger::get(module_path!(), $crate::logging::LogLevel::Info),
                $($arg)*
            );
        }
    })
}

#[macro_export]
#[cfg(target_os = "none")]
macro_rules! warn{
    ( $($arg:tt)* ) => ({
		if $crate::logging::MAX_LEVEL >= $crate::logging::LogLevel::Warn{
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::logging::Logger::get(module_path!(), $crate::logging::LogLevel::Warn),
                $($arg)*
            );
        }
    })
}

#[macro_export]
#[cfg(target_os = "none")]
macro_rules! error{
    ( $($arg:tt)* ) => ({
		if $crate::logging::MAX_LEVEL >= $crate::logging::LogLevel::Error{
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::logging::Logger::get(module_path!(), $crate::logging::LogLevel::Error),
                $($arg)*
            );
        }
    })
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
#[cfg(target_os = "none")]
pub extern "C" fn _start() -> ! {
    crate::trace!("_start()");

    // Initializes the system runtime.
    init();

    cfg_if::cfg_if! {
        if #[cfg(feature = "staticlib")] {
            let status: i32 = c_trampoline();
        } else {
            let status: i32 = rust_trampoline();
        }
    }

    // Cleans up the system runtime.
    cleanup();

    // Exits the runtime.
    let Err(e) = ::sys::kcall::pm::exit(status);
    panic!("failed to exit process manager daemon: {:?}", e);
}

///
/// Trampoline for Rust applications.
///
#[cfg(all(target_os = "none", not(feature = "staticlib")))]
fn rust_trampoline() -> i32 {
    extern "Rust" {
        fn main() -> Result<(), ::sys::error::Error>;
    }

    // Runs the main function.
    match unsafe { main() } {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}

///
/// Trampoline for C applications.
///
#[cfg(all(target_os = "none", feature = "staticlib"))]
fn c_trampoline() -> i32 {
    extern "C" {
        fn main(argc: i32, argv: *const *const u8) -> i32;
        fn _init();
        fn _fini();
    }

    // TODO: set argc and argv.
    let argc: i32 = 1;
    let argv: *const *const u8 = core::ptr::null();

    unsafe {
        _init();
        let ret: i32 = main(argc, argv);
        _fini();
        ret
    }
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
