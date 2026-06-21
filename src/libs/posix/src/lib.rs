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

extern crate alloc;

// Force-link `libc_stdio` into the `libposix.a` staticlib (the `c-main`
// flavour consumed by C executables) so that its `#[no_mangle]` stdio
// symbols — the `stdin`/`stdout`/`stderr` stream accessors and the FILE
// API advertised by the sysroot `<stdio.h>` — are bundled.  This mirrors
// how `syscall` force-links `libc_stdlib` for the staticlib build.
#[cfg(feature = "c-main")]
extern crate libc_stdio;

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
