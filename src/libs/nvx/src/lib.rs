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
mod panic;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "allocator")]
extern crate alloc;

#[cfg(all(target_os = "none", feature = "staticlib"))]
use ::alloc::vec::Vec;

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

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(not(feature = "std"), target_os = "none", not(feature = "staticlib")))]
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

#[no_mangle]
#[cfg(target_os = "none")]
pub extern "C" fn _start(argp: *mut i8, envp: *mut i8) -> ! {
    syslog::trace!("_start(): argv: {:?}, envp: {:?}", argp, envp);

    // Initializes the system runtime.
    init();

    cfg_if::cfg_if! {
        if #[cfg(feature = "staticlib")] {
            let status: i32 = c_trampoline(argp, envp);
        } else {
            let status: i32 = rust_trampoline(argp);
        }
    }

    // Cleans up the system runtime.
    cleanup();

    // Exits the runtime.
    let Err(e) = ::sys::kcall::pm::exit(status);
    panic!("failed to exit process manager daemon: {:?}", e);
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
#[cfg(all(target_os = "none", feature = "staticlib"))]
unsafe fn build_string_table(string: *mut i8) -> ::alloc::vec::Vec<*mut i8> {
    use alloc::vec::Vec;
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
#[cfg(all(target_os = "none", feature = "staticlib"))]
unsafe fn parse_argp(argp: *mut i8) -> ::alloc::vec::Vec<*const i8> {
    build_string_table(argp)
        .into_iter()
        .map(|ptr| ptr as *const i8)
        .collect()
}

///
/// Wrapper for parsing `envp`.
///
#[cfg(all(target_os = "none", feature = "staticlib"))]
unsafe fn parse_envp(envp: *mut i8) -> ::alloc::vec::Vec<*mut i8> {
    build_string_table(envp)
}

///
/// Trampoline for Rust applications.
///
#[cfg(all(target_os = "none", not(feature = "staticlib")))]
fn rust_trampoline(_argp: *mut i8) -> i32 {
    extern "Rust" {
        fn main() -> Result<(), ::sys::error::Error>;
    }

    // Runs the main function.
    match unsafe { main() } {
        Ok(_) => 0,
        Err(e) => e.code.get(),
    }
}

///
/// Trampoline for C applications.
///
#[cfg(all(target_os = "none", feature = "staticlib"))]
fn c_trampoline(argp: *mut i8, envp: *mut i8) -> i32 {
    extern "C" {
        fn main(argc: i32, argv: *const *const u8) -> i32;
        fn _init();
        fn _fini();
    }

    #[allow(non_upper_case_globals)]
    #[no_mangle]
    static mut environ: *mut *mut i8 = core::ptr::null_mut();

    // Build arguments vector.
    let argv: Vec<*const i8> = unsafe { parse_argp(argp) };
    let argc: i32 = argv.len() as i32 - 1;
    let argv: *const *const u8 = argv.as_ptr() as *const *const u8;
    let mut env = unsafe { parse_envp(envp) };
    unsafe {
        environ = env.as_mut_ptr();
    }

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

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
#[cfg(all(target_os = "none", not(feature = "staticlib")))]
pub unsafe extern "C" fn memset(ptr: *mut u8, value: i32, num: usize) -> *mut u8 {
    let mut i: usize = 0;
    while i < num {
        *ptr.add(i) = value as u8;
        i += 1;
    }
    ptr
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
#[cfg(all(target_os = "none", not(feature = "staticlib")))]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, num: usize) -> *mut u8 {
    let mut i: usize = 0;
    while i < num {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
#[cfg(all(target_os = "none", not(feature = "staticlib")))]
pub unsafe extern "C" fn memcmp(ptr1: *const u8, ptr2: *const u8, num: usize) -> i32 {
    let mut i: usize = 0;
    while i < num {
        if *ptr1.add(i) != *ptr2.add(i) {
            return (*ptr1.add(i) as i32) - (*ptr2.add(i) as i32);
        }
        i += 1;
    }
    0
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
#[cfg(all(target_os = "none", not(feature = "staticlib")))]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, num: usize) -> *mut u8 {
    if (dest as *const u8) < src {
        memcpy(dest, src, num)
    } else {
        let mut i: usize = num;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
        dest
    }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
#[cfg(all(target_os = "none", not(feature = "staticlib")))]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
    let mut i: usize = 0;
    while *s.add(i) != 0 {
        i += 1;
    }
    i
}
