// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

extern crate nvx;

// Force `sys-ffi`'s `#[no_mangle]` kernel-call wrappers into the
// `libposix.a` staticlib so other archives that link `libposix` can
// resolve the unmangled `__kcall_*` / `_do_exit_thread` /
// `__kcall_snapshot` / `_do_start_thread` names at link time.  Most
// of these wrappers are Rust ABI (they take and return
// `Result<T, Error>`, `&mut T`, etc.) -- this is about link-time
// symbol resolution, not C callability -- but the unmangled name is
// what the rest of the static-archive graph (libnvx_crt0 thread
// stubs, port-library glue) expects to find.  Without this
// `extern crate`, cargo metadata alone is not enough to force a leaf
// crate with no `pub` re-exports to be linked into the final
// staticlib.
//
// DO NOT REMOVE this `extern crate` line.  It looks unused to tools
// like `cargo fix` or `unused_extern_crates` lints, but removing it
// silently drops every `__kcall_*` symbol from `libposix.a` and
// breaks every executable that resolves these symbols by their
// unmangled name at link time.  Verify any change to this line with:
//   nm <libposix.a> | grep -E ' T (__kcall_|_do_exit_thread|_do_start_thread)' | wc -l
// which must report 36.
#[allow(unused_extern_crates)]
extern crate sys_ffi;

extern crate alloc;

#[cfg(feature = "syscall")]
extern crate syslog;

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

/// Process start-of-day driver called from `nvx-crt0::_start`.
///
/// Owns the stateful runtime services (heap, TDA, argv / envp parsing)
/// that bring up and tear down a Nanvix process.  See the module-level
/// rationale comment in `start.rs` for why these live in libposix
/// instead of in `nvx-crt0`.
pub mod start;

/// File last access and modification times.
pub mod utime;

/// System-specific headers.
pub mod sys;
