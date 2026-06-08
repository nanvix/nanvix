// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![no_std]

//! Nanvix guest runtime helpers.
//!
//! Provides the panic handler, the heap-region setup (`init`), the teardown
//! routine (`cleanup`), and the PIE relocation primitive (`pie`) that every
//! Nanvix guest binary and library needs.
//!
//! The executable entry point (`_do_start` / `_start`) and the C / Rust
//! trampolines that bridge to the application's `main` function live in the
//! sibling `nvx-crt0` crate.  Libraries (notably `libposix`) depend only on
//! this crate; executables depend on both this crate and `nvx-crt0`.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(not(feature = "rustc-dep-of-std"))]
mod panic;
pub mod pie;

//==================================================================================================
// Imports
//==================================================================================================

// We link the `alloc` crate when building static libraries to provide
// heap allocation support.  Gated on the `runtime` feature for the same
// reason as the `init`/`cleanup` functions: consumers that do NOT bring
// up the runtime (notably `nvx-crt0` in its stateless staticlib build)
// must not pull `alloc` into their compilation, because `alloc` requires
// a `#[global_allocator]` which would re-introduce the duplicate-
// `sysalloc` problem.
#[cfg(all(not(feature = "rustc-dep-of-std"), feature = "runtime"))]
extern crate alloc;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Initialises the system runtime.
///
/// Brings up the heap-region reservation, the `sysalloc` allocator, and the
/// thread-data-area (TDA) used for thread-local storage.
///
/// Available only under the `runtime` feature, for consumers that bring up
/// the runtime through `nvx` directly.  The default startup path no longer
/// calls this: `nvx-crt0::_start` tail-calls into libposix's
/// `__nanvix_libc_start_main`, which performs the equivalent bring-up in
/// `posix::start::runtime_init` right after `pie::relocate_pie_binary`.
#[cfg(feature = "runtime")]
pub fn init() {
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    {
        // Reserve virtual address space for the heap from the unified mmap region.
        let heap_capacity: usize = ::config::memory_layout::USER_HEAP_CAPACITY;
        let heap_base: ::sys::mm::VirtualAddress = match sysalloc::vaddr::reserve(heap_capacity) {
            core::prelude::v1::Ok(base) => base,
            Err(e) => panic!("failed to reserve virtual address space for heap: {:?}", e),
        };

        if let Err(e) = sysalloc::init(heap_base, heap_capacity) {
            panic!("failed to initialize memory manager: {:?}", e);
        }
    }
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    match sysalloc::tda::alloc() {
        core::prelude::v1::Ok(Some(tda_ptr)) => {
            if let Err(error) = ::sys::kcall::pm::__kcall_set_thread_data_area(tda_ptr) {
                panic!("init(): failed to set thread data area (error={error:?})");
            }
            sysalloc::tda::mark_initialized();
        },
        core::prelude::v1::Ok(None) => {
            // No thread-local storage to set.
        },
        Err(error) => {
            panic!("init(): create thread data area (error={error:?})");
        },
    }
}

/// Cleans up the system runtime.
///
/// Tears down the thread-data-area and the `sysalloc` allocator.
///
/// Available only under the `runtime` feature, for consumers that bring up
/// the runtime through `nvx` directly.  The default startup path no longer
/// calls this: teardown happens in libposix's `__nanvix_libc_start_main`
/// (`posix::start::runtime_cleanup`) right before the process exits via the
/// `__kcall_exit` syscall.
#[cfg(feature = "runtime")]
pub fn cleanup() {
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    if let Err(error) = ::sysalloc::tda::cleanup() {
        panic!("failed to cleanup thread data area ({error:?})");
    }
    #[cfg(any(target_os = "none", target_os = "nanvix"))]
    if let Err(e) = sysalloc::cleanup() {
        panic!("failed to cleanup memory manager: {:?}", e);
    }
}
