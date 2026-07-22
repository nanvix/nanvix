// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

// Attributes
#![no_std]
// Lints
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]

//==================================================================================================
// Re-exports
//==================================================================================================

/// Pull in all libc_* crate symbols so the linker includes them in the static
/// archive. Gated on `staticlib`: only the produced `libc.a` needs the full C
/// surface forced in. The `rlib` path (nvx-crt0 pulling this crate for the
/// `start` backend) stays lean.
#[cfg(feature = "staticlib")]
extern crate libc_assert;
#[cfg(feature = "staticlib")]
extern crate libc_ctype;
#[cfg(feature = "staticlib")]
extern crate libc_fnmatch;
#[cfg(feature = "staticlib")]
extern crate libc_inttypes;
#[cfg(feature = "staticlib")]
extern crate libc_langinfo;
#[cfg(feature = "staticlib")]
extern crate libc_libgen;
#[cfg(feature = "staticlib")]
extern crate libc_locale;
#[cfg(feature = "staticlib")]
extern crate libc_mntent;
#[cfg(feature = "staticlib")]
extern crate libc_regex;
#[cfg(feature = "staticlib")]
extern crate libc_setjmp;
#[cfg(feature = "staticlib")]
extern crate libc_signal;
#[cfg(feature = "staticlib")]
extern crate libc_stdio;
#[cfg(feature = "staticlib")]
extern crate libc_stdlib;
#[cfg(feature = "staticlib")]
extern crate libc_string;
#[cfg(feature = "staticlib")]
extern crate libc_time;
#[cfg(feature = "staticlib")]
extern crate libc_wchar;
#[cfg(feature = "staticlib")]
extern crate libc_wctype;

/// Heap-backed startup (`start` module) needs `alloc`.
#[cfg(feature = "backend-nanvix")]
extern crate alloc;

/// Runtime support: panic handler (nvx), global allocator (sysalloc), and the
/// process start-of-day driver (the `start` module below). Gated behind the
/// `backend-nanvix` feature so a consumer supplying its own backend (panic
/// handler + global allocator + crt0 + syscalls) can build with
/// `--no-default-features`.
#[cfg(feature = "backend-nanvix")]
extern crate nvx;
#[cfg(feature = "backend-nanvix")]
extern crate sysalloc;

/// POSIX syscall-backed surface. Each `extern crate` forces the linker to
/// include the crate's `#[no_mangle]` C-ABI symbols, exactly as the non-syscall
/// `extern crate libc_*` lines above do. Gated on `backend-nanvix` (which
/// enables `syscall`): the syscall-backed wrappers (`mkdir`, `lstat`, ...) are
/// part of the backend, so they are present both in `libc.a` and on the `rlib`
/// path that `nvx-crt0` pulls for Rust no_std binaries.
#[cfg(feature = "backend-nanvix")]
extern crate libc_arpa_inet;
#[cfg(feature = "backend-nanvix")]
extern crate libc_dirent;
#[cfg(feature = "backend-nanvix")]
extern crate libc_dlfcn;
#[cfg(feature = "backend-nanvix")]
extern crate libc_errno;
#[cfg(feature = "backend-nanvix")]
extern crate libc_ftw;
#[cfg(feature = "backend-nanvix")]
extern crate libc_glob;
#[cfg(feature = "backend-nanvix")]
extern crate libc_grp;
#[cfg(feature = "backend-nanvix")]
extern crate libc_netdb;
#[cfg(feature = "backend-nanvix")]
extern crate libc_poll;
#[cfg(feature = "backend-nanvix")]
extern crate libc_pthread;
#[cfg(feature = "backend-nanvix")]
extern crate libc_pwd;
#[cfg(feature = "backend-nanvix")]
extern crate libc_sys_ioctl;
#[cfg(feature = "backend-nanvix")]
extern crate libc_sys_resource;
#[cfg(feature = "backend-nanvix")]
extern crate libc_sys_stat;
#[cfg(feature = "backend-nanvix")]
extern crate libc_sys_statvfs;
#[cfg(feature = "backend-nanvix")]
extern crate libc_sys_time;
#[cfg(feature = "backend-nanvix")]
extern crate libc_sys_times;
#[cfg(feature = "backend-nanvix")]
extern crate libc_sys_uio;
#[cfg(feature = "backend-nanvix")]
extern crate libc_sys_un;
#[cfg(feature = "backend-nanvix")]
extern crate libc_sys_utsname;
#[cfg(feature = "backend-nanvix")]
extern crate libc_termios;
#[cfg(feature = "backend-nanvix")]
extern crate libc_unistd;
#[cfg(feature = "backend-nanvix")]
extern crate libc_utime;

/// Transitive dependencies required for staticlib resolution.
extern crate sys;
extern crate syslog;

/// Process start-of-day driver called from `nvx-crt0::_start`. Owns the
/// stateful runtime services (heap, TDA, argv / envp parsing) that bring up and
/// tear down a Nanvix process. Moved here from the former `posix` crate so
/// `libc.a` is the only library C applications must link against.
#[cfg(feature = "backend-nanvix")]
pub mod start;

//==================================================================================================
// errno support
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_long,
        c_void,
    },
    iconv::iconv_t,
    sys_types::c_size_t,
};

/// Thread-local errno storage (single-threaded: a plain static suffices).
#[allow(non_upper_case_globals)]
static mut errno_val: c_int = 0;

/// Returns a pointer to the per-thread `errno` variable.
///
/// # Safety
///
/// Returns a mutable pointer to a global variable.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __errno() -> *mut c_int {
    &raw mut errno_val
}

//==================================================================================================
// Signal disposition (sigaction) backend required by libc_signal
//==================================================================================================

// `libc_signal::signal::sigaction_t` (the C ABI view) and `sys::pm::SigAction` (the kernel-call
// view) are not textually identical — `sa_sigaction` is `Option<extern "C" fn(...)>` on the libc
// side but `usize` in `SigAction` (pointer-sized via the null-pointer optimization). The pointer
// casts in `sigaction` below are only sound while the two stay ABI-compatible, so guard the size,
// alignment, and field offsets at compile time; any future divergence fails the build instead of
// silently corrupting the ABI.
#[cfg(feature = "backend-nanvix")]
const _: () = {
    use ::libc_signal::signal::sigaction_t;
    use ::sys::pm::SigAction;
    assert!(::core::mem::size_of::<sigaction_t>() == ::core::mem::size_of::<SigAction>());
    assert!(::core::mem::align_of::<sigaction_t>() == ::core::mem::align_of::<SigAction>());
    assert!(
        ::core::mem::offset_of!(sigaction_t, sa_handler)
            == ::core::mem::offset_of!(SigAction, sa_handler)
    );
    assert!(
        ::core::mem::offset_of!(sigaction_t, sa_mask)
            == ::core::mem::offset_of!(SigAction, sa_mask)
    );
    assert!(
        ::core::mem::offset_of!(sigaction_t, sa_flags)
            == ::core::mem::offset_of!(SigAction, sa_flags)
    );
    assert!(
        ::core::mem::offset_of!(sigaction_t, sa_sigaction)
            == ::core::mem::offset_of!(SigAction, sa_sigaction)
    );
};

/// `sigaction` — installs and/or queries the disposition of a signal via the `sigaction()` kernel
/// call. This is the backend symbol that `libc_signal`'s `signal()` shim links against.
///
/// `libc_signal::signal::sigaction_t` and `sys::pm::SigAction` are ABI-compatible (same `repr(C)`
/// size, alignment, and field offsets for `sa_handler`, `sa_mask`, `sa_flags`, `sa_sigaction`,
/// asserted at compile time above), so the action pointers are reinterpreted across the boundary
/// without translation.
///
/// # Safety
///
/// This function writes to the errno location and dereferences raw pointers. `act` and `oldact`
/// must be either null or valid, properly aligned pointers to `sigaction_t` values.
#[cfg(feature = "backend-nanvix")]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigaction(
    signum: c_int,
    act: *const libc_signal::signal::sigaction_t,
    oldact: *mut libc_signal::signal::sigaction_t,
) -> c_int {
    match unsafe {
        ::sys::kcall::pm::__kcall_sigaction(
            signum,
            act as *const ::sys::pm::SigAction,
            oldact as *mut ::sys::pm::SigAction,
        )
    } {
        Ok(()) => 0,
        Err(error) => {
            *__errno() = error.code.get();
            -1
        },
    }
}

/// Stub `sigaction` for builds that supply their own backend (no kernel-call surface available).
///
/// # Safety
///
/// This function writes to the errno location and dereferences raw pointers.
#[cfg(not(feature = "backend-nanvix"))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn sigaction(
    _signum: c_int,
    _act: *const libc_signal::signal::sigaction_t,
    _oldact: *mut libc_signal::signal::sigaction_t,
) -> c_int {
    *__errno() = ::sysapi::errno::ENOSYS;
    -1
}

//==================================================================================================
// Signal mask (sigprocmask) backend required by libc_signal
//==================================================================================================

/// `sigprocmask` backend gets and/or modifies the calling thread's blocked-signal mask via the
/// `sigprocmask()` kernel call.
///
/// # Safety
///
/// This function writes to the errno location and dereferences raw pointers. `set` and `oldset`
/// must be either null or valid, properly aligned pointers to `sigset_t` values.
#[cfg(feature = "backend-nanvix")]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __nanvix_sigprocmask(
    how: c_int,
    set: *const libc_signal::signal::sigset_t,
    oldset: *mut libc_signal::signal::sigset_t,
) -> c_int {
    match unsafe {
        ::sys::kcall::pm::__kcall_sigprocmask(
            how,
            set as *const ::sys::pm::SigSet,
            oldset as *mut ::sys::pm::SigSet,
        )
    } {
        Ok(()) => 0,
        Err(error) => {
            *__errno() = error.code.get();
            -1
        },
    }
}

/// Stub `sigprocmask` backend for builds that supply their own backend.
///
/// # Safety
///
/// This function writes to the errno location and dereferences raw pointers.
#[cfg(not(feature = "backend-nanvix"))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __nanvix_sigprocmask(
    _how: c_int,
    _set: *const libc_signal::signal::sigset_t,
    _oldset: *mut libc_signal::signal::sigset_t,
) -> c_int {
    *__errno() = ::sysapi::errno::ENOSYS;
    -1
}

//==================================================================================================
// Signal pending/suspend (sigpending/sigsuspend) backends required by libc_signal
//==================================================================================================

/// `sigpending` backend reports the calling thread's pending-but-blocked signal set via the
/// `sigpending()` kernel call.
///
/// # Safety
///
/// This function writes to the errno location and dereferences `set`, which must be a valid,
/// properly aligned pointer to a `sigset_t`.
#[cfg(feature = "backend-nanvix")]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __nanvix_sigpending(set: *mut libc_signal::signal::sigset_t) -> c_int {
    match unsafe { ::sys::kcall::pm::__kcall_sigpending(set as *mut ::sys::pm::SigSet) } {
        Ok(()) => 0,
        Err(error) => {
            *__errno() = error.code.get();
            -1
        },
    }
}

/// Stub `sigpending` backend for builds that supply their own backend.
///
/// # Safety
///
/// This function writes to the errno location and dereferences raw pointers.
#[cfg(not(feature = "backend-nanvix"))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __nanvix_sigpending(_set: *mut libc_signal::signal::sigset_t) -> c_int {
    *__errno() = ::sysapi::errno::ENOSYS;
    -1
}

/// `sigsuspend` backend installs `mask` and suspends until a caught signal is delivered via the
/// `sigsuspend()` kernel call. It always returns `-1`; on the expected path `errno` is `EINTR`.
///
/// # Safety
///
/// This function writes to the errno location and dereferences `mask`, which must be a valid,
/// properly aligned pointer to a `sigset_t`.
#[cfg(feature = "backend-nanvix")]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __nanvix_sigsuspend(mask: *const libc_signal::signal::sigset_t) -> c_int {
    match unsafe { ::sys::kcall::pm::__kcall_sigsuspend(mask as *const ::sys::pm::SigSet) } {
        // sigsuspend() never succeeds; this arm is unreachable but keeps the match total.
        Ok(()) => 0,
        Err(error) => {
            *__errno() = error.code.get();
            -1
        },
    }
}

/// Stub `sigsuspend` backend for builds that supply their own backend.
///
/// # Safety
///
/// This function writes to the errno location and dereferences raw pointers.
#[cfg(not(feature = "backend-nanvix"))]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __nanvix_sigsuspend(_mask: *const libc_signal::signal::sigset_t) -> c_int {
    *__errno() = ::sysapi::errno::ENOSYS;
    -1
}

/// Wrapper to make a raw pointer `Sync` for use as a static.
#[allow(dead_code)]
#[repr(transparent)]
pub struct SyncPtr(*mut c_void);
unsafe impl Sync for SyncPtr {}

/// C++ ABI: DSO handle symbol required by __cxa_atexit.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub static __dso_handle: SyncPtr = SyncPtr(core::ptr::null_mut());

//==================================================================================================
// Time zone globals (fixed UTC; Nanvix has no time-zone database)
//==================================================================================================

/// Backing storage for the `tzname` strings. Mutable so that any write a C
/// consumer performs through `tzname` targets writable memory (writing through
/// a pointer derived from an immutable `static` would be undefined behavior).
static mut TZ_UTC: [c_char; 4] = [b'U' as c_char, b'T' as c_char, b'C' as c_char, 0];

/// Standard/daylight time zone name strings. Always reports UTC.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub static mut tzname: [*mut c_char; 2] = [
    (&raw mut TZ_UTC).cast::<c_char>(),
    (&raw mut TZ_UTC).cast::<c_char>(),
];

/// Seconds west of UTC. Always zero (UTC).
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub static mut timezone: c_long = 0;

/// Daylight-saving flag. Always zero (no DST).
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub static mut daylight: c_int = 0;

/// `tzset` — initializes the time-zone globals. They are already fixed to UTC, so this is a no-op.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tzset() {}

//==================================================================================================
// System logging stubs (no system log daemon in standalone mode)
//==================================================================================================

/// Stub `openlog` — no system log daemon, so this is a no-op.
///
/// # Safety
///
/// `ident` may be null or a valid C string; it is not dereferenced.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn openlog(_ident: *const c_char, _option: c_int, _facility: c_int) {}

/// Stub `closelog` — no-op.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn closelog() {}

/// Stub `syslog` — discards the message (declared variadic in C; the extra
/// arguments are ignored under the cdecl caller-cleanup ABI).
///
/// # Safety
///
/// `format` may be null or a valid C string; it is not dereferenced.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn syslog(_priority: c_int, _format: *const c_char) {}

/// Stub `setlogmask` — tracks nothing and reports an empty previous mask.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn setlogmask(_mask: c_int) -> c_int {
    0
}

//==================================================================================================
// Generic C11/GCC atomic library routines
//==================================================================================================
//
// Clang emits calls to these generic `__atomic_*` helpers for atomic operations
// it cannot lower inline (arbitrary sizes / unknown alignment). The Nanvix guest
// process is single-threaded for the purposes of these operations, so plain
// (non-atomic) memory accesses are sufficient; the memory-order arguments are
// ignored.

/// Atomically loads `size` bytes from `mem` into `ret`.
///
/// # Safety
///
/// `mem` and `ret` must point to at least `size` bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __atomic_load(
    size: c_size_t,
    mem: *const c_void,
    ret: *mut c_void,
    _model: c_int,
) {
    unsafe { core::ptr::copy_nonoverlapping(mem.cast::<u8>(), ret.cast::<u8>(), size as usize) };
}

/// Atomically stores `size` bytes from `val` into `mem`.
///
/// # Safety
///
/// `mem` and `val` must point to at least `size` bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __atomic_store(
    size: c_size_t,
    mem: *mut c_void,
    val: *const c_void,
    _model: c_int,
) {
    unsafe { core::ptr::copy_nonoverlapping(val.cast::<u8>(), mem.cast::<u8>(), size as usize) };
}

/// Atomically exchanges `size` bytes: copies `mem` into `ret`, then `val` into `mem`.
///
/// # Safety
///
/// `mem`, `val`, and `ret` must point to at least `size` bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __atomic_exchange(
    size: c_size_t,
    mem: *mut c_void,
    val: *const c_void,
    ret: *mut c_void,
    _model: c_int,
) {
    let n: usize = size as usize;
    unsafe {
        core::ptr::copy_nonoverlapping(mem.cast::<u8>(), ret.cast::<u8>(), n);
        core::ptr::copy_nonoverlapping(val.cast::<u8>(), mem.cast::<u8>(), n);
    }
}

/// Atomic compare-and-exchange of `size` bytes.
///
/// Returns `true` and stores `desired` into `mem` when `mem` equals `expected`; otherwise updates
/// `expected` with the current contents of `mem` and returns `false`.
///
/// # Safety
///
/// `mem`, `expected`, and `desired` must point to at least `size` bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __atomic_compare_exchange(
    size: c_size_t,
    mem: *mut c_void,
    expected: *mut c_void,
    desired: *const c_void,
    _success: c_int,
    _failure: c_int,
) -> bool {
    let n: usize = size as usize;
    let m: *const u8 = mem.cast::<u8>();
    let e: *const u8 = expected.cast::<u8>();
    let mut i: usize = 0;
    while i < n {
        if unsafe { *m.add(i) } != unsafe { *e.add(i) } {
            // Mismatch: report the current value in `expected`.
            unsafe { core::ptr::copy_nonoverlapping(m, expected.cast::<u8>(), n) };
            return false;
        }
        i += 1;
    }
    unsafe { core::ptr::copy_nonoverlapping(desired.cast::<u8>(), mem.cast::<u8>(), n) };
    true
}

/// Reports whether an atomic of the given `size` is lock-free.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn __atomic_is_lock_free(size: c_size_t, _ptr: *const c_void) -> bool {
    matches!(size, 1 | 2 | 4 | 8)
}

//==================================================================================================
// Codeset conversion (identity passthrough)
//==================================================================================================

/// Opens an identity codeset-conversion descriptor (no real conversion is performed).
///
/// # Safety
///
/// `tocode` and `frocode` may be null or valid C strings; they are not dereferenced.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn iconv_open(_tocode: *const c_char, _fromcode: *const c_char) -> iconv_t {
    // A non-null, never-dereferenced sentinel handle. `iconv_open` reports
    // failure as `(iconv_t)-1`, so any non-`-1`, non-null value signals success;
    // `dangling_mut` yields such a pointer (address == align_of::<c_void>() == 1).
    core::ptr::dangling_mut::<c_void>()
}

/// Copies bytes from the input to the output buffer unchanged (identity conversion).
///
/// # Safety
///
/// The buffer pointers and counters must be valid for the indicated lengths.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn iconv(
    _cd: iconv_t,
    inbuf: *mut *mut c_char,
    inbytesleft: *mut c_size_t,
    outbuf: *mut *mut c_char,
    outbytesleft: *mut c_size_t,
) -> c_size_t {
    // A null/!inbuf request resets state; nothing to do for identity.
    if inbuf.is_null() || (*inbuf).is_null() {
        return 0;
    }

    let in_left: c_size_t = if inbytesleft.is_null() {
        0
    } else {
        *inbytesleft
    };
    let out_left: c_size_t = if outbytesleft.is_null() {
        0
    } else {
        *outbytesleft
    };
    let n: c_size_t = core::cmp::min(in_left, out_left);

    core::ptr::copy_nonoverlapping(*inbuf, *outbuf, n as usize);
    *inbuf = (*inbuf).add(n as usize);
    *outbuf = (*outbuf).add(n as usize);
    if !inbytesleft.is_null() {
        *inbytesleft = in_left - n;
    }
    if !outbytesleft.is_null() {
        *outbytesleft = out_left - n;
    }

    // If input remains, the output buffer was too small.
    if in_left > n {
        *__errno() = ::sysapi::errno::E2BIG;
        return c_size_t::MAX;
    }
    0
}

/// Closes an identity codeset-conversion descriptor.
///
/// # Safety
///
/// `cd` is the handle returned by [`iconv_open`]; it is not dereferenced.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn iconv_close(_cd: iconv_t) -> c_int {
    0
}
