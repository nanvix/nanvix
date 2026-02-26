// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(never_type)] // pthread requires this.
#![feature(c_variadic)] // fcntl requires this.

//==================================================================================================
// Modules
//==================================================================================================

extern crate nvx;

extern crate alloc;

#[cfg(feature = "syscall")]
extern crate syslog;

// Force inclusion of memfs C-callable symbols in `libposix.a`.
// The `#[used]` attribute on function pointers and `core::hint::black_box`
// prevent the compiler from dead-stripping the wrapper functions.
#[cfg(feature = "memfs")]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn memfs_init_from_ramfs(
    mount_path: *const ::core::ffi::c_char,
) -> ::sysapi::ffi::c_int {
    ::core::hint::black_box(::syscall::memfs::memfs_init_from_ramfs(mount_path))
}

#[cfg(feature = "memfs")]
#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn memfs_file_size(path: *const ::core::ffi::c_char) -> i64 {
    ::core::hint::black_box(::syscall::memfs::memfs_file_size(path))
}

// Address and routing parameter area.
pub mod arpa;

/// Dynamic linking.
pub mod dlfcn;

/// Dummy implementations.
pub mod dummy;

/// System error numbers.
pub mod errno;

/// Virtual environments.
pub mod venv;

/// Definitions for network database operations.
pub mod netdb;

/// Definitions for the poll() function.
pub mod poll;

/// Posix threads.
pub mod pthread;

/// Password structure.
pub mod pwd;

/// File last access and modification times.
pub mod utime;

/// System-specific headers.
pub mod sys;
