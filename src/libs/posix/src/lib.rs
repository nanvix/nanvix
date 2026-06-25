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

#[cfg(feature = "syscall")]
extern crate syslog;

// POSIX modules that have been split into standalone `libc_*` crates and are
// re-exported by this bundle.  The `extern crate` declarations force the linker
// to include each crate's `#[no_mangle]` C-ABI symbols in `libposix.a`, exactly
// as the `extern crate libc_*` lines do in the `nanvix_libc` bundle.  They are
// gated on the `syscall` feature because those crates provide the syscall-backed
// libc surface, matching the feature that previously gated these modules'
// logging dependency.

/// Internet address manipulation (`arpa/inet.h`).
#[cfg(feature = "syscall")]
extern crate libc_arpa_inet;

/// Dynamic linking (`dlfcn.h`).
#[cfg(feature = "syscall")]
extern crate libc_dlfcn;

/// System error numbers (`errno.h`).
#[cfg(feature = "syscall")]
extern crate libc_errno;

/// Network database operations (`netdb.h`).
#[cfg(feature = "syscall")]
extern crate libc_netdb;

/// The poll() function (`poll.h`).
#[cfg(feature = "syscall")]
extern crate libc_poll;

/// POSIX threads (`pthread.h`).
#[cfg(feature = "syscall")]
extern crate libc_pthread;

/// Password database (`pwd.h`).
#[cfg(feature = "syscall")]
extern crate libc_pwd;

/// File last access and modification times (`utime.h`).
#[cfg(feature = "syscall")]
extern crate libc_utime;

/// I/O control operations (`sys/ioctl.h`).
#[cfg(feature = "syscall")]
extern crate libc_sys_ioctl;

/// Resource operations (`sys/resource.h`).
#[cfg(feature = "syscall")]
extern crate libc_sys_resource;

/// File status (`sys/stat.h`).
#[cfg(feature = "syscall")]
extern crate libc_sys_stat;

/// Time types (`sys/time.h`).
#[cfg(feature = "syscall")]
extern crate libc_sys_time;

/// Process times (`sys/times.h`).
#[cfg(feature = "syscall")]
extern crate libc_sys_times;

/// UNIX domain socket addresses (`sys/un.h`).
#[cfg(feature = "syscall")]
extern crate libc_sys_un;

/// Vector I/O operations (`sys/uio.h`).
#[cfg(feature = "syscall")]
extern crate libc_sys_uio;

/// System name (`sys/utsname.h`).
#[cfg(feature = "syscall")]
extern crate libc_sys_utsname;

/// Dummy implementations.
pub mod dummy;

/// Group database.
pub mod grp;

/// Virtual environments.
pub mod venv;

/// Process start-of-day driver called from `nvx-crt0::_start`.
///
/// Owns the stateful runtime services (heap, TDA, argv / envp parsing)
/// that bring up and tear down a Nanvix process.  See the module-level
/// rationale comment in `start.rs` for why these live in libposix
/// instead of in `nvx-crt0`.
pub mod start;
