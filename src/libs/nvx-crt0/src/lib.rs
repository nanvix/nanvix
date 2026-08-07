// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![forbid(clippy::large_stack_frames)]
#![forbid(clippy::large_stack_arrays)]
#![no_std]

//! Nanvix guest C runtime 0 (`crt0`) — stateless startup shim.
//!
//! Owns the kernel-entry asm stub (`_do_start`), the minimal Rust `_start`
//! function it dispatches to, the C / Rust trampolines that bridge to the
//! application's `main` function, and the `ARGC` / `ARGV` statics that
//! Rust no_std binaries can read.
//!
//! This crate is intentionally **stateless** in the sysroot staticlib build
//! (`forwarding-allocator`): it carries no `sysalloc` objects and does not
//! allocate; it only provides a tiny forwarding `#[global_allocator]` stub.
//! All stateful runtime services live in `libc`'s `start` module and are
//! reached via the `extern "C" fn __nanvix_libc_start_main` entry point.
//!
//! # Architectural rationale
//!
//! This split mirrors the glibc / musl / newlib / picolibc convention:
//! `crt1.o` / `crt0.o` is a tiny stateless wrapper; the libc-side function
//! (`__libc_start_main`) carries all stateful work.  In Nanvix terms,
//! `libnvx_crt0.a` is the wrapper and `libc.a` is the libc.
//!
//! The split is what makes the design correct.  Before this layout
//! `nvx-crt0` itself depended on `sysalloc` (via `nvx::init`) AND used a
//! `Vec` for argv parsing, so `libnvx_crt0.a` carried a copy of
//! `sysalloc`'s state — including `VADDR_NEXT` — that was a DISTINCT
//! cargo compilation from the one inside `libc.a`.  Two
//! `VADDR_NEXT` globals meant `dlopen()` collided with the already-
//! mapped heap pages.  See
//! `nanvix-todo/dlopen-load-address-conflict.md`.

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

// Exactly one allocator strategy must be selected.  See the matching
// section in `Cargo.toml` for the rationale of the split: cargo-lib
// consumers (Rust no_std binaries) get `provides-allocator`; the
// `cargo rustc --crate-type staticlib` build that produces
// `libnvx_crt0.a` for cpython uses `forwarding-allocator`.
#[cfg(all(feature = "provides-allocator", feature = "forwarding-allocator"))]
compile_error!(
    "nvx-crt0: features `provides-allocator` and `forwarding-allocator` are mutually exclusive; \
     pick exactly one."
);

//==================================================================================================
// Imports
//==================================================================================================

use ::config::memory_layout::USER_BASE_RAW;
use ::core::ffi::c_char;

// When `provides-allocator` is enabled, force `sysalloc` into the
// final binary's link graph so its `#[global_allocator]` is picked up
// by rustc.  Without this `extern crate`, rustc fails the binary's
// compile with "no global memory allocator found" because no other
// source line references `sysalloc` (the binary depends on it as a
// transitive cargo dep but never names it).
//
// This is the cargo-lib path used by the 29 Rust no_std binaries.
// The `libnvx_crt0.a` staticlib build (cpython) goes through
// `--no-default-features --features "c-main forwarding-allocator"`
// instead, which leaves `provides-allocator` OFF so no `sysalloc`
// objects end up in the cpython link via `libnvx_crt0.a`.
#[cfg(feature = "provides-allocator")]
extern crate sysalloc;

// Pull `nanvix_libc` into the binary's link graph so libc-side
// `__nanvix_libc_start_main` (and the C-ABI bridges it relies on)
// resolve at link time.  Same gating reason as `extern crate sysalloc`
// above.
#[cfg(feature = "provides-allocator")]
extern crate nanvix_libc;

#[cfg(feature = "rust-main")]
use ::core::sync::atomic::{
    AtomicI32,
    AtomicPtr,
    Ordering,
};

//==================================================================================================
// External Functions
//==================================================================================================

// `__nanvix_libc_start_main` is the libc-side process startup driver
// (analogous to glibc's `__libc_start_main`).  It owns runtime init
// (heap, TDA), argv / envp parsing, environment-table population, the
// trampoline call (via `__nanvix_main` below), runtime cleanup, and
// process exit.  See `nanvix_libc/src/start.rs`.
unsafe extern "C" {
    fn __nanvix_libc_start_main(argp: *mut c_char, envp: *mut c_char) -> !;
}

// Raw `sysalloc` C-ABI bridges exported by `libc` (see
// `nanvix_libc/src/start.rs`).  Used as the implementation of the
// forwarding `#[global_allocator]` below (only built when the
// `forwarding-allocator` feature is on); they call `sysalloc::alloc`
// / `sysalloc::dealloc` directly so there is no recursion through
// Rust's global-allocator slot.
#[cfg(feature = "forwarding-allocator")]
unsafe extern "C" {
    fn __nanvix_rust_alloc_raw(size: usize, align: usize) -> *mut u8;
    fn __nanvix_rust_dealloc_raw(ptr: *mut u8, size: usize, align: usize);
}

//==================================================================================================
// Global Allocator (forwarding, staticlib only)
//==================================================================================================

// The Rust `alloc` crate (pulled in unconditionally by `-Zbuild-std=
// core,alloc`) requires every staticlib that links it to declare a
// `#[global_allocator]`, even when the staticlib's own code never
// allocates.  This crate has no allocation sites of its own — all
// allocating work happens inside `libc.a` — so we provide a
// minimal forwarding `#[global_allocator]` that defers to libc's
// `sysalloc` via the C-ABI bridges above.
//
// The forwarder is gated on the `forwarding-allocator` feature
// because for Rust no_std binaries, `sysalloc` itself provides
// `#[global_allocator]` (via `nvx`'s `runtime` feature, on by default)
// and a second one declared here would conflict at compile time.

#[cfg(feature = "forwarding-allocator")]
struct LibcAllocator;

#[cfg(feature = "forwarding-allocator")]
unsafe impl ::core::alloc::GlobalAlloc for LibcAllocator {
    unsafe fn alloc(&self, layout: ::core::alloc::Layout) -> *mut u8 {
        unsafe { __nanvix_rust_alloc_raw(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: ::core::alloc::Layout) {
        unsafe { __nanvix_rust_dealloc_raw(ptr, layout.size(), layout.align()) };
    }
}

#[cfg(feature = "forwarding-allocator")]
#[global_allocator]
static ALLOCATOR: LibcAllocator = LibcAllocator;

//==================================================================================================
// Global Variables
//==================================================================================================

///
/// # Description
///
/// Number of command-line arguments published by `__nanvix_libc_start_main` for
/// Rust no_std executables that wish to read them directly (for example
/// `cmdline-len-rust` / `cmdline-env-rust-nostd`).
///
/// Only meaningful when the `rust-main` feature is enabled. C executables
/// receive `argc` directly through the C trampoline.
///
#[cfg(feature = "rust-main")]
pub static ARGC: AtomicI32 = AtomicI32::new(0);

///
/// # Description
///
/// Pointer to the command-line argument array published by
/// `__nanvix_libc_start_main`.  See [`ARGC`] for the matching length.
///
#[cfg(feature = "rust-main")]
pub static ARGV: AtomicPtr<*const u8> = AtomicPtr::new(::core::ptr::null_mut());

//==================================================================================================
// Kernel-Entry Stub
//==================================================================================================

// Kernel-entry stub set up by the process spawner. The trap frame the kernel
// installs makes IRET "return" to `_do_start` with `argp` in EDX and `envp`
// in ECX. The stub aligns the stack to the i386 SysV ABI requirement
// (ESP = 0 mod 16 before the CALL instruction) and dispatches to `_start`.
#[cfg(target_arch = "x86")]
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
    # Safety net: _start() calls __nanvix_libc_start_main() which calls
    # exit() and never returns.  If control somehow falls through, spin
    # forever rather than running into whatever follows in memory.
    1:  jmp 1b
    "#
);

// x86_64 variant: the kernel enters user mode (`__leave_kernel_to_user_mode` + `iretq`) with the
// SysV argument registers already set up — RDI = argp, RSI = envp — and the user RSP pointing at
// the top of the freshly forged user stack (no arguments are placed on the stack itself). The stub
// therefore must NOT read argp/envp from the stack; it only realigns RSP to the SysV 16-byte
// boundary and dispatches to `_start(argp, envp)`.
#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    r#"
    .extern _start

    .globl _do_start

    .section .crt0, "ax"

    _do_start:
        and rsp, -16
        mov rbp, rsp
        call _start
    1:  jmp 1b
    "#
);

// AArch64 variant: the kernel enters EL0 with X0 = argp, X1 = envp, and a 16-byte-aligned user
// stack, which already matches AAPCS64.
#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    r#"
    .extern _start

    .globl _do_start
    .type _do_start, @function

    .section .crt0, "ax"

    _do_start:
        mov x29, sp
        bl _start
    1:  b 1b
    "#
);

//==================================================================================================
// Signal-Return Trampoline
//==================================================================================================

// Position-independent restorer trampoline. The kernel installs this address as the return address
// of a caught-signal handler frame, so that when a handler returns it lands here and issues the
// `sigreturn()` kernel call. `sigreturn()` restores the interrupted context and resumes it
// directly, so control never returns to the `jmp` safety net below.
//
// The trampoline only loads the kernel-call number and traps; `sigreturn()` locates the signal
// frame from the user stack pointer, so no register arguments are required.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
core::arch::global_asm!(
    r#"
    .globl __nvx_sigreturn_trampoline
    .type __nvx_sigreturn_trampoline, @function

    .section .text, "ax"

    __nvx_sigreturn_trampoline:
        mov eax, {sigreturn_nr}
        int {kcall_vector}
    1:  jmp 1b
    "#,
    sigreturn_nr = const ::sys::number::KcallNumber::Sigreturn as u32,
    kcall_vector = const ::sys::number::KCALL_VECTOR,
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    r#"
    .globl __nvx_sigreturn_trampoline
    .type __nvx_sigreturn_trampoline, @function

    .section .text, "ax"

    __nvx_sigreturn_trampoline:
        mov w8, #{sigreturn_nr}
        svc #{kcall_vector}
    1:  b 1b
    "#,
    sigreturn_nr = const ::sys::number::KcallNumber::Sigreturn as u32,
    kcall_vector = const ::sys::number::KCALL_VECTOR,
);

// Address-only view of the trampoline symbol defined in assembly above, used to register the
// restorer with the kernel without forming a function pointer (which would require an
// `fn`-to-integer cast).
unsafe extern "C" {
    static __nvx_sigreturn_trampoline: u8;
}

//==================================================================================================
// Rust Entry Point
//==================================================================================================

///
/// # Description
///
/// Rust entry point reached from `_do_start`.  Performs PIE relocation and
/// then immediately tail-calls into libc's `__nanvix_libc_start_main`,
/// which owns runtime init, argv / envp parsing, trampoline dispatch,
/// cleanup, and process exit.
///
/// This function intentionally does NOT allocate, log, or otherwise touch
/// the heap — the heap is brought up by `__nanvix_libc_start_main`.  This
/// is what keeps `libnvx_crt0.a` free of `sysalloc` state.  See the
/// module-level doc comment.
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
/// the kernel trap-frame setup, and it relies on `__nanvix_libc_start_main`
/// being provided at link time by `libc.a`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start(argp: *mut c_char, envp: *mut c_char) -> ! {
    // Apply PIE relocations before any global data access.  Must happen
    // here (not in libc) because the libc-side `__nanvix_libc_start_main`
    // accesses global statics (sysalloc, environ, ...) that themselves
    // need relocation.
    unsafe {
        ::nvx::pie::relocate_pie_binary(USER_BASE_RAW);
    }

    // Register the position-independent signal-return trampoline so the kernel can deliver caught
    // signals to this image. Done after relocation so the symbol address is the final runtime one,
    // and best-effort: a failure here only disables signal handlers, not startup. Re-registering
    // here also re-resolves the restorer after `execv()` replaces the image.
    let restorer: usize = ::core::ptr::addr_of!(__nvx_sigreturn_trampoline) as usize;
    let _ = ::sys::kcall::pm::__kcall_sig_restorer(restorer);

    unsafe {
        __nanvix_libc_start_main(argp, envp);
    }
}

//==================================================================================================
// Application Trampoline
//==================================================================================================

///
/// # Description
///
/// C-ABI trampoline invoked by libc.a `__nanvix_libc_start_main` to
/// dispatch into the application's entry point.
///
/// In `c-main` mode (cpython, hello-c, smoke, ...), this calls the
/// standard `extern "C" fn main(int, char **)`.  By default it also invokes
/// the legacy GCC-style `_init` / `_fini` constructor / destructor hooks
/// around `main`, preserving the historical contract that C consumers rely
/// on.  When the `init-array` feature is enabled — as it is for the in-tree
/// `nanvix_libc` bundle — those hooks are dropped and global constructors /
/// destructors are instead run by `__nanvix_libc_start_main`, which walks
/// `.init_array` / `.fini_array` around this trampoline.
///
/// In `rust-main` mode (every Rust no_std binary in this workspace),
/// this publishes the parsed `argc` / `argv` into [`ARGC`] / [`ARGV`]
/// (so application code can read them) and then calls the Rust-side
/// `extern "Rust" fn main() -> Result<(), sys::error::Error>`.
///
/// # Parameters
///
/// - `argc`: Parsed argument count, supplied by `__nanvix_libc_start_main`.
/// - `argv`: Parsed argument vector, supplied by `__nanvix_libc_start_main`.
///
/// # Returns
///
/// The exit status to be passed to `__kcall_exit`.
///
/// # Safety
///
/// This function dereferences the foreign `argv` pointer and calls into the
/// foreign `main` (and, unless the `init-array` feature is enabled, the
/// foreign `_init` / `_fini`) provided by the application (resolved at link
/// time).
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nanvix_main(argc: i32, argv: *const *const u8) -> i32 {
    cfg_if::cfg_if! {
        if #[cfg(feature = "c-main")] {
            unsafe extern "C" {
                fn main(argc: i32, argv: *const *const u8) -> i32;
            }
            cfg_if::cfg_if! {
                if #[cfg(feature = "init-array")] {
                    // Global constructors / destructors are run separately by
                    // `__nanvix_libc_start_main` (libc.a start.rs), which
                    // walks `.init_array` / `.fini_array` around this
                    // trampoline.  Matches the in-tree `nanvix_libc` bundle,
                    // which ships no crti/crtn/crtbegin/crtend and therefore
                    // provides no `_init` / `_fini`.
                    unsafe { main(argc, argv) }
                } else {
                    // Default (backward-compatible) path: invoke the legacy
                    // GCC-style `_init` / `_fini` constructor / destructor
                    // hooks (provided by the application's crti/crtn) around
                    // `main`, preserving the historical contract for C
                    // consumers that depend on it.
                    unsafe extern "C" {
                        fn _init();
                        fn _fini();
                    }
                    let _ = argc;
                    let _ = argv;
                    unsafe {
                        _init();
                        let ret: i32 = main(argc, argv);
                        _fini();
                        ret
                    }
                }
            }
        } else {
            unsafe extern "Rust" {
                fn main() -> Result<(), ::sys::error::Error>;
            }
            ARGC.store(argc, Ordering::SeqCst);
            ARGV.store(argv as *mut *const u8, Ordering::SeqCst);
            match unsafe { main() } {
                Ok(()) => 0,
                Err(e) => e.code.get(),
            }
        }
    }
}
