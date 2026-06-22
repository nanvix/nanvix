// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

//! Nanvix C math library (`libm.a`).
//!
//! This crate is a thin staticlib aggregator. It pulls in the pure-computational
//! math routines from `libc_math` (`sin`, `cos`, `pow`, `sqrt`, `floor`, ...) and
//! the Nanvix `#[panic_handler]` from `nvx`. Because `nvx` is built with
//! `default-features = false`, its `runtime` feature is disabled and it brings in
//! NO `sysalloc`/`#[global_allocator]` — `libm.a` is therefore self-contained and
//! shares no mutable runtime state with `libc.a`.
//!
//! Together with `libc.a` and `libnvx_crt0.a`, `libm.a` forms a full replacement
//! for the GCC + newlib `crt0` + `libc.a` + `libm.a` toolchain surface.

// Attributes
#![no_std]

//==================================================================================================
// Re-exports
//==================================================================================================

/// Pull in all `libc_math` symbols so the linker includes them in the static
/// archive (the same mechanism `nanvix_libc` uses for the `libc_*` crates).
extern crate libc_math;

/// Provides the `#[panic_handler]`. With `default-features = false`, `nvx` links
/// in only the panic handler — its heap/TDA bring-up and the transitive
/// `sysalloc`/`alloc` dependencies are gated behind the (disabled) `runtime`
/// feature, so no `#[global_allocator]` is pulled into `libm.a`.
extern crate nvx;

//==================================================================================================
// Global Allocator (forwarding)
//==================================================================================================

// The Rust `alloc` crate (pulled in unconditionally by `-Zbuild-std=core,alloc`)
// requires every staticlib that links it to declare a `#[global_allocator]`,
// even though the math routines never allocate. Declaring a real `sysalloc`
// allocator here would embed a SECOND `sysalloc` instance (with its own `HEAP`
// static) alongside the one in `libc.a` — the exact duplicate-allocator bug this
// project fights. Instead we forward to `libc.a`'s allocator through the same
// C-ABI bridges `nvx-crt0` uses; the symbols resolve at the final link and are
// never actually called from `libm.a`.
unsafe extern "C" {
    fn __nanvix_rust_alloc_raw(size: usize, align: usize) -> *mut u8;
    fn __nanvix_rust_dealloc_raw(ptr: *mut u8, size: usize, align: usize);
}

struct ForwardingAllocator;

unsafe impl ::core::alloc::GlobalAlloc for ForwardingAllocator {
    unsafe fn alloc(&self, layout: ::core::alloc::Layout) -> *mut u8 {
        unsafe { __nanvix_rust_alloc_raw(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: ::core::alloc::Layout) {
        unsafe { __nanvix_rust_dealloc_raw(ptr, layout.size(), layout.align()) };
    }
}

#[global_allocator]
static ALLOCATOR: ForwardingAllocator = ForwardingAllocator;
