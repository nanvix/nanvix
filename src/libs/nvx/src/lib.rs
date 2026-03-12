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
mod pie;

//==================================================================================================
// Imports
//==================================================================================================

// We link the `alloc` crate when building static libraries to provide heap allocation support.
#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

use ::config::memory_layout::USER_BASE_RAW;

/// Re-export the VFS crate when the `standalone` feature is enabled.
#[cfg(feature = "standalone")]
pub use ::vfs;

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
    // Apply PIE relocations before any global data access.
    unsafe {
        pie::relocate_pie_binary(USER_BASE_RAW);
    }

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

    // Initialize in-memory filesystem from RAMFS MMIO region (if present).
    #[cfg(feature = "standalone")]
    vfs_init_ramfs();
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

//==================================================================================================
// In-Memory Filesystem Initialization
//==================================================================================================

/// Initializes the VFS and mounts the RAMFS MMIO region at the root (`/`).
///
/// Called automatically during guest startup when the `standalone` feature is
/// enabled. The RAMFS MMIO region is provided by the hypervisor via a
/// well-known tag. If no RAMFS region is present, this function silently
/// returns (the guest may not have been launched with `-ramfs`).
///
/// The MMIO region is mapped with read-write permissions by the kernel, so
/// the filesystem is mounted directly on the MMIO memory without copying.
/// The MMIO allocation is kept for the lifetime of the process (automatic
/// cleanup on exit) so the mounted image remains valid. The image is
/// mounted at `/` so it serves as the root filesystem and relative paths
/// resolve naturally against it.
#[cfg(feature = "standalone")]
fn vfs_init_ramfs() {
    use ::sys::{
        mm::Address,
        pm::Capability,
    };

    /// Encoded 8-byte "RAMFS   " tag exposed by the MicroVM RAMFS MMIO region.
    const RAMFS_MMIO_TAG: u64 = u64::from_be_bytes(*b"RAMFS   ");

    /// Mount path for the RAMFS image (root filesystem).
    const RAMFS_MOUNT_PATH: &str = "/";

    // Initialize the VFS (idempotent — ignore AlreadyInitialized).
    if ::vfs::init().is_err() && !::vfs::is_initialized() {
        ::syslog::warn!("vfs_init_ramfs(): failed to initialize VFS");
        return;
    }

    // Acquire IO management capability.
    if ::sys::kcall::pm::capctl(Capability::IoManagement, true).is_err() {
        ::syslog::warn!("vfs_init_ramfs(): failed to acquire IoManagement capability");
        return;
    }

    // Attempt to allocate and mount the RAMFS MMIO region.
    let mounted: bool = (|| -> bool {
        if ::sys::kcall::mm::mmio_alloc(RAMFS_MMIO_TAG).is_err() {
            // No RAMFS region available — the guest was simply not launched with `-ramfs`.
            return false;
        }

        let info: ::sys::mm::MmioRegionInfo = match ::sys::kcall::mm::mmio_info(RAMFS_MMIO_TAG) {
            Ok(i) => i,
            Err(_) => {
                // Free the MMIO mapping on failure.
                let _ = ::sys::kcall::mm::mmio_free(RAMFS_MMIO_TAG);
                return false;
            },
        };
        let total_size: usize = info.size();
        let base_ptr: *mut u8 = info.base().into_raw_value() as *mut u8;

        // Mount the FAT image directly from the MMIO region (mapped read-write by the kernel).
        // The MMIO allocation is kept for the process lifetime so the image remains valid.
        if unsafe { ::vfs::mount_image(RAMFS_MOUNT_PATH, base_ptr, total_size) }.is_err() {
            ::syslog::warn!("vfs_init_ramfs(): failed to mount RAMFS image");
            let _ = ::sys::kcall::mm::mmio_free(RAMFS_MMIO_TAG);
            return false;
        }

        true
    })();

    // Release IO management capability.
    let _ = ::sys::kcall::pm::capctl(Capability::IoManagement, false);

    if mounted {
        ::syslog::info!("vfs_init_ramfs(): mounted RAMFS at {}", RAMFS_MOUNT_PATH);
    }
}
