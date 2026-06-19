// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Process start-of-day driver — libposix-owned.
//!
//! Owns the stateful runtime services that bring up a Nanvix process
//! and tear it down:
//!
//! - Heap virtual-address reservation, `sysalloc` allocator bring-up.
//! - Thread-data-area (TDA) allocation and install via the matching kcall.
//! - `argp` / `envp` parsing into Vec-backed `argv` / `environ` arrays.
//! - Environment-table population (`__nanvix_env_init`) so
//!   `getenv` / `setenv` / `unsetenv` see the process environment.
//! - Trampoline dispatch into the application's `main`
//!   (via `__nanvix_main`, provided by `nvx-crt0` per binary).
//! - Runtime teardown after `main` returns.
//! - Process exit via `__kcall_exit`.
//!
//! # Architectural rationale
//!
//! This module exists so that the stateless `nvx-crt0` startup crate
//! does NOT need to depend on `sysalloc` or `extern crate alloc` — and
//! therefore `libnvx_crt0.a` carries no `sysalloc` objects.  All
//! heap state (`sysalloc::vaddr::VADDR_NEXT`, ...) is then owned
//! exclusively by `libposix.a`, structurally preventing the duplicated-
//! `VADDR_NEXT` / dlopen-collision bug documented in
//! `nanvix-todo/dlopen-load-address-conflict.md`.
//!
//! The pattern mirrors the glibc / musl / newlib / picolibc convention
//! where `crt1.o` / `crt0.o` is a stateless wrapper and the libc-side
//! `__libc_start_main` carries all stateful work.
//!
//! # Entry-point contract with `nvx-crt0`
//!
//! `nvx-crt0::_start` calls into [`__nanvix_libc_start_main`] (defined
//! here) immediately after PIE relocation.  The reverse call back into
//! `nvx-crt0` for the trampoline dispatch happens via the C-ABI
//! [`__nanvix_main`] declared as an extern below — `nvx-crt0` provides
//! it as a `#[no_mangle]` symbol with `c-main` or `rust-main` semantics
//! selected by feature at the binary's link.

//==================================================================================================
// Imports
//==================================================================================================

#![cfg(all(feature = "allocator", feature = "syscall"))]
use ::alloc::vec::Vec;
use ::config::memory_layout::{
    USER_BASE_RAW,
    USER_HEAP_CAPACITY,
};
use ::core::ffi::c_char;

//==================================================================================================
// External Functions
//==================================================================================================

unsafe extern "C" {
    /// Application trampoline provided by `nvx-crt0`.  Dispatches to
    /// either the C `main(int, char **)` (cpython, hello-c, smoke, ...)
    /// or the Rust `main() -> Result<(), Error>` (every Rust no_std
    /// binary), depending on the trampoline feature chosen at the
    /// binary's link.
    fn __nanvix_main(argc: i32, argv: *const *const u8) -> i32;

    /// Initialises the environment table from a null-terminated array of
    /// "KEY=VALUE" C strings.  Provided by the syscall layer (the `syscall`
    /// crate's stdlib bindings) when the syscall feature set is enabled;
    /// declared as an extern here so this module doesn't need to depend on
    /// the symbol's defining Rust module.
    fn __nanvix_env_init(envp: *const *const c_char);
}

//==================================================================================================
// Global Variables
//==================================================================================================

///
/// # Description
///
/// Pointer to the environment-variable array installed by
/// [`__nanvix_libc_start_main`].
///
/// # Notes
///
/// - This symbol is not name-mangled so it can be referenced from foreign code (for example C).
/// - The symbol name is lowercase because external languages expect this conventional name.
///
#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
pub static mut environ: *mut *mut c_char = ::core::ptr::null_mut();

//==================================================================================================
// Process Start-of-Day Driver
//==================================================================================================

///
/// # Description
///
/// libposix-side process startup driver, analogous to glibc's
/// `__libc_start_main`.  Called from `nvx-crt0::_start` immediately
/// after PIE relocation.
///
/// Performs runtime init, argv / envp parsing, environment-table
/// population, the trampoline call into the application's `main`,
/// runtime cleanup, and the `__kcall_exit` that terminates the process.
///
/// # Parameters
///
/// - `argp`: kernel-installed pointer to a null-terminated, space-separated
///   argument string.  The buffer is mutated in-place (spaces are
///   replaced with NULs) and the resulting `argv` array points into it.
/// - `envp`: same encoding as `argp`, but for environment variables.
///
/// # Returns
///
/// This function does not return.
///
/// # Safety
///
/// - The caller must have completed PIE relocation before invoking this
///   function (otherwise sysalloc / environ statics are at the wrong
///   address).
/// - `argp` and `envp` must point to writable, null-terminated string
///   buffers that remain valid for the lifetime of the process.
/// - `__nanvix_main` must be linked in by the binary (provided by
///   `nvx-crt0`).
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nanvix_libc_start_main(argp: *mut c_char, envp: *mut c_char) -> ! {
    ::syslog::trace!("__nanvix_libc_start_main(): argp={:?}, envp={:?}", argp, envp);

    // Suppress unused-import warning when neither USER_BASE_RAW nor any
    // other consumer needs it; it is referenced only by the `_start`
    // documentation cross-reference in `nvx-crt0`.
    let _ = USER_BASE_RAW;

    // Bring up the stateful runtime (heap, TDA).  All `sysalloc` state
    // lives in this compilation unit (libposix); no other static
    // archive in the final link may depend on `sysalloc`.
    runtime_init();

    // Parse the kernel-supplied argv / envp blobs.  Now safe because
    // the heap is up.
    //
    // The backing vectors are intentionally **leaked** (via
    // `Box::leak` of the boxed slice).  The kernel-installed `argp` /
    // `envp` buffers themselves live for the lifetime of the process,
    // and C code observes the `argv` / `environ` arrays we build here
    // as raw pointers that must remain dereferenceable until process
    // exit.  Dropping the vectors after `main()` returns would leave
    // those pointers dangling for any C destructor / `atexit` handler
    // (or for the libposix-side environment-table consumers below)
    // that still reads them during teardown.  Leaking matches glibc's
    // behaviour for `argv` / `environ`.
    let argv: &'static [*const c_char] =
        ::alloc::boxed::Box::leak(unsafe { parse_argp(argp) }.into_boxed_slice());
    let argc: i32 = argv.len() as i32 - 1;
    let argv_ptr: *const *const u8 = argv.as_ptr() as *const *const u8;

    let env: &'static mut [*mut c_char] =
        ::alloc::boxed::Box::leak(unsafe { parse_envp(envp) }.into_boxed_slice());
    unsafe {
        environ = env.as_mut_ptr();
    }

    // Populate the libposix environment table used by
    // getenv() / setenv() / unsetenv().  The `environ` pointer is a
    // null-terminated array of "KEY=VALUE" C strings, which is exactly
    // the format expected by `__nanvix_env_init`.
    //
    // NOTE: `setenv` / `unsetenv` update only `env_table` (the
    // libposix-side storage); they do NOT rebuild `environ`.  C code
    // that mutates the environment via these functions and then reads
    // back `extern char **environ` will observe a snapshot taken at
    // process start, not the mutated state.  Keeping these two views
    // in sync is left for a follow-up that introduces an
    // `env_table`-backed `environ` proxy.
    unsafe {
        __nanvix_env_init(environ as *const *const c_char);
    }

    // Synchronize with the process daemons before any application code runs. A successful `execv`
    // replaces the image in place and transfers control here without notifying `vfsd`, so the
    // inherited descriptor table still carries its `FD_CLOEXEC` descriptors and this image's
    // resolution cache was wiped with BSS. The barrier holds this process until `vfsd` has dropped
    // those descriptors, so the cache — rebuilt lazily on first use — observes the
    // post-close-on-exec table. In run modes without a guest `vfsd`, this is a no-op.
    ::syscall::unistd::exec_startup_barrier();

    // Dispatch into the application's `main` via the binary-supplied
    // trampoline.  `__nanvix_main` is `nvx-crt0::__nanvix_main`, which
    // selects between the C and Rust trampoline at the binary's link
    // based on which crt0 feature is active.
    let status: i32 = unsafe { __nanvix_main(argc, argv_ptr) };

    // Tear down the runtime.  `argv` / `env` are leaked above (see the
    // comment at their declaration) so no Vec destructors run here
    // that would touch the heap after `runtime_cleanup` releases it.
    runtime_cleanup();

    // Exit the process.  Never returns under normal circumstances.
    let Err(error) = ::sys::kcall::pm::__kcall_exit(status);
    panic!("__nanvix_libc_start_main(): exit kcall returned (error={error:?})");
}

//==================================================================================================
// Runtime Init / Cleanup
//==================================================================================================

/// Reserves the heap virtual-address range and brings up the `sysalloc`
/// allocator + TDA.  Called once at process start by
/// [`__nanvix_libc_start_main`].
fn runtime_init() {
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    {
        let heap_capacity: usize = USER_HEAP_CAPACITY;
        let heap_base: ::sys::mm::VirtualAddress = match ::sysalloc::vaddr::reserve(heap_capacity) {
            ::core::result::Result::Ok(base) => base,
            ::core::result::Result::Err(e) => {
                panic!("runtime_init(): failed to reserve virtual address space for heap: {:?}", e)
            },
        };

        if let Err(e) = ::sysalloc::init(heap_base, heap_capacity) {
            panic!("runtime_init(): failed to initialize memory manager: {:?}", e);
        }
    }
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    match ::sysalloc::tda::alloc() {
        ::core::result::Result::Ok(Some(tda_ptr)) => {
            if let Err(error) = ::sys::kcall::pm::__kcall_set_thread_data_area(tda_ptr) {
                panic!("runtime_init(): failed to set thread data area (error={error:?})");
            }
            ::sysalloc::tda::mark_initialized();
        },
        ::core::result::Result::Ok(None) => {
            // No thread-local storage to set.
        },
        ::core::result::Result::Err(error) => {
            panic!("runtime_init(): create thread data area (error={error:?})");
        },
    }
}

/// Tears down the TDA and the `sysalloc` allocator.  Called once at
/// process shutdown by [`__nanvix_libc_start_main`].
fn runtime_cleanup() {
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    if let Err(error) = ::sysalloc::tda::cleanup() {
        panic!("runtime_cleanup(): failed to cleanup thread data area ({error:?})");
    }
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    if let Err(e) = ::sysalloc::cleanup() {
        panic!("runtime_cleanup(): failed to cleanup memory manager: {:?}", e);
    }
}

//==================================================================================================
// Forwarding-allocator C-ABI bridge
//==================================================================================================

// These two `extern "C"` entry points are the destination of the
// forwarding `#[global_allocator]` defined in `nvx-crt0` when its
// `forwarding-allocator` feature is enabled.
//
// They are deliberately implemented in terms of `sysalloc`'s DIRECT
// Rust API (`sysalloc::alloc` / `sysalloc::dealloc`), NOT in terms of
// `alloc::alloc::alloc` / `alloc::alloc::dealloc` — calling those would
// loop back through Rust's `#[global_allocator]` slot, which in the
// final cpython link is bound to the `nvx-crt0` forwarder, causing
// infinite recursion.
//
// This indirection only exists for the cpython staticlib case.  Rust
// no_std binaries get `sysalloc`'s `#[global_allocator]` directly via
// `nvx`'s `runtime` feature (cargo unifies that single compilation
// into the binary) and never call into these wrappers.

///
/// # Description
///
/// C-ABI bridge that performs a raw allocation through `sysalloc`'s
/// direct API.  Used as the implementation of the forwarding
/// `#[global_allocator]` in `nvx-crt0` (cpython staticlib case).
///
/// # Parameters
///
/// - `size`: Allocation size, in bytes (must satisfy the alignment invariant below).
/// - `align`: Allocation alignment, in bytes (must be a power of two).
///
/// # Returns
///
/// A pointer to the allocation, or null on failure.
///
/// # Safety
///
/// `size` must be a multiple of `align`, and `align` must be a non-zero
/// power of two.  These match the invariants of `core::alloc::Layout`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nanvix_rust_alloc_raw(size: usize, align: usize) -> *mut u8 {
    let layout = match ::core::alloc::Layout::from_size_align(size, align) {
        Ok(l) => l,
        Err(_) => return ::core::ptr::null_mut(),
    };
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    unsafe {
        ::sysalloc::alloc(layout)
    }
    #[cfg(not(any(target_os = "none", target_os = "nanvix")))]
    {
        let _ = layout;
        ::core::ptr::null_mut()
    }
}

///
/// # Description
///
/// C-ABI bridge that releases a raw allocation through `sysalloc`'s
/// direct API.  Companion of [`__nanvix_rust_alloc_raw`].
///
/// # Parameters
///
/// - `ptr`: Pointer previously returned by `__nanvix_rust_alloc_raw`.
/// - `size`: Original allocation size.
/// - `align`: Original allocation alignment.
///
/// # Safety
///
/// `ptr`, `size`, and `align` must match a previous successful
/// `__nanvix_rust_alloc_raw` call.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nanvix_rust_dealloc_raw(ptr: *mut u8, size: usize, align: usize) {
    let layout = match ::core::alloc::Layout::from_size_align(size, align) {
        Ok(l) => l,
        Err(_) => return,
    };
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    unsafe {
        ::sysalloc::dealloc(ptr, layout)
    }
    #[cfg(not(any(target_os = "none", target_os = "nanvix")))]
    {
        let _ = (ptr, layout);
    }
}

//==================================================================================================
// Argument / Environment Parsing
//==================================================================================================

///
/// # Description
///
/// Builds a string table from a null-terminated, space-separated string,
/// rewriting interior spaces to NUL and producing a null-terminated Vec
/// of pointers into the (now-tokenised) buffer.
///
/// # Parameters
///
/// - `string`: A pointer to a null-terminated string.
///
/// # Returns
///
/// A vector of pointers to null-terminated strings.
///
/// # Safety
///
/// This function dereferences `string`.  The caller must ensure that
/// `string` points to a valid mutable buffer terminated by a single
/// null byte and containing only ASCII characters (space-separated
/// tokens).  The buffer must remain valid for as long as the returned
/// pointers are used.
///
unsafe fn build_string_table(string: *mut c_char) -> Vec<*mut c_char> {
    use ::core::ptr;

    let mut current = string;
    let mut count = 0;

    // Traverse `current`, replacing spaces with null characters and counting entries.
    while unsafe { *current } != 0 {
        if unsafe { *current } == b' ' as c_char {
            unsafe { *current = b'\0' as c_char };
            count += 1;
        }
        current = unsafe { current.add(1) };
    }
    count += 1; // Account for the null-terminator.

    // Create an array of pointers to the entries.
    let mut result: Vec<*mut c_char> = Vec::with_capacity(count as usize + 1);
    current = string;
    for _ in 0..count {
        ::syslog::trace!("build_string_table(): entry[{}]: {:?}", result.len(), unsafe {
            ::core::ffi::CStr::from_ptr(current)
        });

        result.push(current);
        while unsafe { *current } != 0 {
            current = unsafe { current.add(1) };
        }
        current = unsafe { current.add(1) }; // Skip the null terminator.
    }
    result.push(ptr::null_mut()); // Null-terminate the array.

    result
}

/// Wrapper for parsing `argp`.
///
/// # Safety
///
/// See [`build_string_table`].
unsafe fn parse_argp(argp: *mut c_char) -> Vec<*const c_char> {
    unsafe { build_string_table(argp) }
        .into_iter()
        .map(|ptr| ptr as *const c_char)
        .collect()
}

/// Wrapper for parsing `envp`.
///
/// # Safety
///
/// See [`build_string_table`].
unsafe fn parse_envp(envp: *mut c_char) -> Vec<*mut c_char> {
    unsafe { build_string_table(envp) }
}
