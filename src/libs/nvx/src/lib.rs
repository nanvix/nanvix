// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![no_std]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(not(feature = "rustc-dep-of-std"))]
mod panic;

//==================================================================================================
// Imports
//==================================================================================================

// We link the `alloc` crate when building static libraries to provide heap allocation support.
#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

#[cfg(not(feature = "staticlib"))]
use ::core::sync::atomic::{
    AtomicI32,
    AtomicPtr,
    Ordering,
};

use ::alloc::vec::Vec;

//==================================================================================================
// Global Variables
//==================================================================================================

///
/// # Description
///
/// Pointer to environment variables.
///
/// # Note
///
/// - This symbol is not name-mangled so it can be referenced from foreign code (for example C).
/// - The symbol name is lowercase because external languages expect this conventional name.
///
#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
static mut environ: *mut *mut i8 = core::ptr::null_mut();

///
/// # Description
///
/// Pointer to command line arguments.
///
#[cfg(not(feature = "staticlib"))]
pub static ARGV: AtomicPtr<*const u8> = AtomicPtr::new(core::ptr::null_mut());

///
/// # Description
///
/// Number of command line arguments in the `argv` array.
///
#[cfg(not(feature = "staticlib"))]
pub static ARGC: AtomicI32 = AtomicI32::new(0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(not(feature = "staticlib"))]
core::arch::global_asm!(
    r#"
    .extern _start

    .globl _do_start

    .section .crt0, "ax"

    _do_start:
        mov ebp, esp
        push ecx
        push edx
        call _start
    1:  jmp 1b
    "#
);

///
/// # Description
///
/// Entry point of the program.
///
/// # Parameters
///
/// - `argp`: A pointer to a null-terminated string containing the program arguments.
/// - `envp`: A pointer to a null-terminated string containing the environment variables.
///
/// # Returns
///
/// This function does not return.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start(argp: *mut i8, envp: *mut i8) -> ! {
    syslog::trace!("_start(): argv: {:?}, envp: {:?}", argp, envp);

    // Initializes the system runtime.
    init();

    // Build vector of command line arguments.
    let argv: Vec<*const i8> = unsafe { parse_argp(argp) };
    let argc: i32 = argv.len() as i32 - 1;
    let argv: *mut *const u8 = argv.as_ptr() as *mut *const u8;
    #[cfg(not(feature = "staticlib"))]
    {
        ARGC.store(argc, Ordering::SeqCst);
        ARGV.store(argv, Ordering::SeqCst);
    }

    // Build vector of environment variables.
    let mut env: Vec<*mut i8> = unsafe { parse_envp(envp) };
    unsafe {
        environ = env.as_mut_ptr();
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "staticlib")] {
            let status: i32 = c_trampoline(argc, argv);
        } else {
            let status: i32 = rust_trampoline();
        }
    }

    // Cleans up the system runtime.
    cleanup();

    // Exits the runtime.
    let Err(error) = ::sys::kcall::pm::exit(status);
    panic!("failed to exit process (error={error:?})");
}

///
/// # Description
///
/// Builds a string table from a a null-terminated string.
///
/// # Parameters
///
/// - `string`: A pointer to a null-terminated string.
///
/// # Returns
///
/// - A vector of pointers to null-terminated strings.
///
unsafe fn build_string_table(string: *mut i8) -> Vec<*mut i8> {
    use core::ptr;

    let mut current = string;
    let mut count = 0;

    // Traverse `current`, replacing spaces with null characters and counting entries.
    while *current != 0 {
        if *current == b' ' as i8 {
            *current = b'\0' as i8;
            count += 1;
        }
        current = current.add(1);
    }
    count += 1; // Account for the null-terminator.

    // Create an array of pointers to the entries.
    let mut result: Vec<*mut i8> = Vec::with_capacity(count as usize);
    current = string;
    for _ in 0..count {
        // Print the current entry.
        ::syslog::trace!(
            "build_string_table(): entry[{}]: {:?}",
            result.len(),
            // Convert to CStr for printing.
            ::core::ffi::CStr::from_ptr(current)
        );

        result.push(current);
        while *current != 0 {
            current = current.add(1);
        }
        current = current.add(1); // Skip the null terminator.
    }
    result.push(ptr::null_mut()); // Null-terminate the array.

    result
}

///
/// Wrapper for parsing `argp`.
///
unsafe fn parse_argp(argp: *mut i8) -> Vec<*const i8> {
    build_string_table(argp)
        .into_iter()
        .map(|ptr| ptr as *const i8)
        .collect()
}

///
/// Wrapper for parsing `envp`.
///
unsafe fn parse_envp(envp: *mut i8) -> Vec<*mut i8> {
    build_string_table(envp)
}

///
/// Trampoline for Rust applications.
///
#[cfg(all(not(feature = "staticlib"), not(feature = "rustc-dep-of-std")))]
fn rust_trampoline() -> i32 {
    unsafe extern "Rust" {
        fn main() -> Result<(), ::sys::error::Error>;
    }

    // Runs the main function.
    match unsafe { main() } {
        Ok(()) => 0,
        Err(e) => e.code.get(),
    }
}

///
/// Trampoline for Rust applications.
///
#[cfg(all(not(feature = "staticlib"), feature = "rustc-dep-of-std"))]
fn rust_trampoline() -> i32 {
    unsafe extern "Rust" {
        fn main();
    }

    // Runs the main function.
    unsafe { main() };

    0
}

///
/// Trampoline for C applications.
///
#[cfg(feature = "staticlib")]
fn c_trampoline(argc: i32, argv: *const *const u8) -> i32 {
    unsafe extern "C" {
        fn main(argc: i32, argv: *const *const u8) -> i32;
        fn _init();
        fn _fini();
    }

    unsafe {
        _init();
        let ret: i32 = main(argc, argv);
        _fini();
        ret
    }
}

/// Initializes system runtime.
fn init() {
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    if let Err(e) = sysalloc::init() {
        panic!("failed to initialize memory manager: {:?}", e);
    }
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    match sysalloc::tda::alloc() {
        core::prelude::v1::Ok(Some(tda_ptr)) => {
            if let Err(error) = ::sys::kcall::pm::set_thread_data_area(tda_ptr) {
                panic!("init(): failed to set thread data area (error={error:?})");
            }
        },
        core::prelude::v1::Ok(None) => {
            // No thread-local storage to set.
        },
        Err(error) => {
            panic!("init(): create thread data area (error={error:?})");
        },
    }
}

/// Cleans up system runtime.
fn cleanup() {
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    if let Err(error) = ::sysalloc::tda::cleanup() {
        panic!("failed to cleanup thread data area ({error:?})");
    }
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    if let Err(e) = sysalloc::cleanup() {
        panic!("failed to cleanup memory manager: {:?}", e);
    }
}
