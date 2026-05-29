// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![no_std]

//! Nanvix guest C runtime 0 (`crt0`).
//!
//! Owns the executable entry point (`_do_start`), the `_start` Rust function
//! it dispatches to, the C / Rust trampolines that bridge to the
//! application's `main` function, and the argument-vector / environment
//! parsing logic.  This crate is intended to be linked **only into
//! executables**.  Libraries (notably `libposix.a`) must **not** depend on
//! it, so that those libraries can be linked into `.so` files without
//! pulling in a strong undefined `main` reference.

//==================================================================================================
// Feature-set guards
//==================================================================================================

// Exactly one trampoline flavour must be selected. The C variant brings in
// `extern "C" fn main(int, char **)` (the standard C ABI entry point used by
// CPython, hello-c, smoke, ...); the Rust variant brings in
// `extern "Rust" fn main() -> Result<(), sys::error::Error>` (used by every
// Rust no_std binary).
#[cfg(all(feature = "c-main", feature = "rust-main"))]
compile_error!(
    "nvx-crt0: features `c-main` and `rust-main` are mutually exclusive; pick exactly one."
);

#[cfg(not(any(feature = "c-main", feature = "rust-main")))]
compile_error!("nvx-crt0: one of the features `c-main` or `rust-main` must be enabled.");

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::alloc::vec::Vec;
use ::config::memory_layout::USER_BASE_RAW;

#[cfg(feature = "rust-main")]
use ::core::sync::atomic::{
    AtomicI32,
    AtomicPtr,
    Ordering,
};

//==================================================================================================
// External Functions
//==================================================================================================

#[cfg(feature = "c-main")]
unsafe extern "C" {
    /// Initialises the environment table from a null-terminated array of "KEY=VALUE"
    /// C strings.  Provided by `libposix` (`src/libs/posix/src/stdlib/...`) and only
    /// needed by C executables, which rely on `getenv` / `setenv` / `unsetenv`.
    fn __nanvix_env_init(envp: *const *const core::ffi::c_char);
}

//==================================================================================================
// Global Variables
//==================================================================================================

///
/// # Description
///
/// Pointer to the environment-variable array installed by `_start()`.
///
/// # Notes
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
/// Number of command-line arguments published by `_start()` for Rust no_std
/// executables that wish to read them directly (for example
/// `cmdline-len-rust` / `cmdline-env-rust-nostd`).
///
/// Only meaningful when the `rust-main` feature is enabled. C executables
/// receive `argc` directly through `c_trampoline`.
///
#[cfg(feature = "rust-main")]
pub static ARGC: AtomicI32 = AtomicI32::new(0);

///
/// # Description
///
/// Pointer to the command-line argument array published by `_start()`. See
/// [`ARGC`] for the matching length.
///
#[cfg(feature = "rust-main")]
pub static ARGV: AtomicPtr<*const u8> = AtomicPtr::new(core::ptr::null_mut());

//==================================================================================================
// Kernel-Entry Stub
//==================================================================================================

// Kernel-entry stub set up by the process spawner. The trap frame the kernel
// installs makes IRET "return" to `_do_start` with `argp` in EDX and `envp`
// in ECX. The stub aligns the stack to the i386 SysV ABI requirement
// (ESP = 0 mod 16 before the CALL instruction) and dispatches to `_start`.
core::arch::global_asm!(
    r#"
    .extern _start

    .globl _do_start

    .section .crt0, "ax"

    _do_start:
        #
        # Entry point for newly created processes.
        #
        # The kernel sets up a trap frame so that IRET "returns" to this function.
        # The kernel passes the argument pointer in EDX and the environment pointer
        # in ECX.
        #
        # This stub must satisfy the i386 SysV ABI calling convention before
        # invoking _start(argp, envp):
        #  - Arguments are pushed right-to-left (envp first, then argp).
        #  - At the CALL instruction, ESP must be 0 mod 16, so the return address
        #    push leaves the callee with ESP = 12 (mod 16).
        #
        # Stack alignment arithmetic:
        #   and esp,-16 -> ESP = 0 (mod 16)   (force 16-byte alignment)
        #   mov ebp,esp -> set frame pointer for the process root frame
        #   sub esp, 8  -> ESP = 8 (mod 16)   (alignment padding)
        #   push ecx    -> ESP = 4 (mod 16)   (push envp -- second parameter)
        #   push edx    -> ESP = 0 (mod 16)   (push argp -- first parameter)
        #   call        -> ESP = 12 (mod 16)  (return address pushed by CALL)
        #
        and esp, -16
        mov ebp, esp
        sub esp, 8
        push ecx
        push edx
        call _start
    # Safety net: _start() calls exit() and never returns.
    # If it somehow does, spin forever rather than falling through.
    1:  jmp 1b
    "#
);

//==================================================================================================
// Rust Entry Point
//==================================================================================================

///
/// # Description
///
/// Rust entry point reached from `_do_start`.  Performs PIE relocation, runs
/// `nvx`'s runtime initialisation, parses the kernel-supplied `argp` /
/// `envp` blobs into argv / environ arrays, dispatches to the selected
/// trampoline (C or Rust `main`), then tears the runtime down and exits via
/// the `exit` kcall.
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
/// This function is unsafe because it dereferences raw pointers supplied by
/// the kernel trap-frame setup.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start(argp: *mut i8, envp: *mut i8) -> ! {
    // Apply PIE relocations before any global data access.
    unsafe {
        ::nvx::pie::relocate_pie_binary(USER_BASE_RAW);
    }

    ::syslog::trace!("_start(): argv: {:?}, envp: {:?}", argp, envp);

    // Initialise the system runtime (heap, TDA, ...).
    ::nvx::init();

    // Build vector of command-line arguments.
    let mut argv_vec: Vec<*const i8> = unsafe { parse_argp(argp) };
    let argc: i32 = argv_vec.len() as i32 - 1;
    let argv: *mut *const u8 = argv_vec.as_mut_ptr() as *mut *const u8;
    #[cfg(feature = "rust-main")]
    {
        ARGC.store(argc, Ordering::SeqCst);
        ARGV.store(argv, Ordering::SeqCst);
    }

    // Build vector of environment variables.
    let mut env: Vec<*mut i8> = unsafe { parse_envp(envp) };
    unsafe {
        environ = env.as_mut_ptr();
    }

    // Populate the environment table used by getenv()/setenv()/unsetenv().
    // The `environ` pointer is a null-terminated array of "KEY=VALUE" C
    // strings, which is exactly the format expected by __nanvix_env_init().
    #[cfg(feature = "c-main")]
    unsafe {
        __nanvix_env_init(environ as *const *const core::ffi::c_char);
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "c-main")] {
            let status: i32 = c_trampoline(argc, argv);
        } else {
            // Suppress unused-variable warnings when only the Rust trampoline
            // is active: argc/argv are stored in ARGC/ARGV above.
            let _ = (argc, argv);
            let status: i32 = rust_trampoline();
        }
    }

    // Clean up the system runtime.
    ::nvx::cleanup();

    // Exit the runtime.
    let Err(error) = ::sys::kcall::pm::__kcall_exit(status);
    panic!("failed to exit process (error={error:?})");
}

//==================================================================================================
// Trampolines
//==================================================================================================

/// Trampoline for Rust applications.
///
/// Calls the `main` symbol exported by the application crate, which must
/// have the signature `fn main() -> Result<(), sys::error::Error>`.
#[cfg(feature = "rust-main")]
fn rust_trampoline() -> i32 {
    unsafe extern "Rust" {
        fn main() -> Result<(), ::sys::error::Error>;
    }

    match unsafe { main() } {
        Ok(()) => 0,
        Err(e) => e.code.get(),
    }
}

/// Trampoline for C applications.
///
/// Calls the standard C `main(int argc, char **argv) -> int` plus `_init`
/// and `_fini` (the legacy GNU constructor/destructor hooks).
#[cfg(feature = "c-main")]
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

//==================================================================================================
// Argument / Environment Parsing
//==================================================================================================

///
/// # Description
///
/// Builds a string table from a null-terminated string.
/// # Parameters
///
/// - `string`: A pointer to a null-terminated string.
///
/// # Returns
///
/// - A vector of pointers to null-terminated strings.
///
/// # Safety
///
/// This function dereferences `string`.  The caller must ensure that `string`
/// points to a valid mutable buffer terminated by a single null byte and
/// containing only ASCII characters (space-separated tokens).
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

/// Wrapper for parsing `argp`.
///
/// # Safety
///
/// See [`build_string_table`].
unsafe fn parse_argp(argp: *mut i8) -> Vec<*const i8> {
    unsafe { build_string_table(argp) }
        .into_iter()
        .map(|ptr| ptr as *const i8)
        .collect()
}

/// Wrapper for parsing `envp`.
///
/// # Safety
///
/// See [`build_string_table`].
unsafe fn parse_envp(envp: *mut i8) -> Vec<*mut i8> {
    unsafe { build_string_table(envp) }
}
