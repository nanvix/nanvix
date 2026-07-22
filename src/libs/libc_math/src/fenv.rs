// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

pub use ::sysapi::fenv::{
    fenv_t,
    fexcept_t,
    FE_ALL_EXCEPT,
    FE_DENORMAL,
    FE_DFL_ENV,
    FE_DIVBYZERO,
    FE_DOWNWARD,
    FE_INEXACT,
    FE_INVALID,
    FE_OVERFLOW,
    FE_TONEAREST,
    FE_TOWARDZERO,
    FE_UNDERFLOW,
    FE_UPWARD,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Math functions report errors through `errno` when this bit is set.
pub const MATH_ERRNO: c_int = 1;
/// Math functions report errors through floating-point exceptions when this bit is set.
pub const MATH_ERREXCEPT: c_int = 2;
/// Error-reporting mechanisms implemented by the math library.
pub const MATH_ERRHANDLING: c_int = 0;

/// Mask covering all rounding-mode bits.
const FE_ROUND_MASK: c_int = 0x0c00;

//==================================================================================================
// Rounding-Mode Storage
//==================================================================================================

// Tracks the current rounding mode for `fegetround`/`fesetround` consistency.
//
// POSIX specifies the floating-point environment as per-thread state, so hosted
// (`std`) builds keep the mode in thread-local storage. The freestanding
// (`no_std`) build uses an atomic, which provides race-free shared access
// without relying on a thread-local runtime. Floating-point code generated for
// this target uses SSE, whose default round-to-nearest matches `FE_TONEAREST`.

macro_rules! define_round_storage {
    () => {
        #[cfg(feature = "std")]
        mod round {
            use ::core::cell::Cell;
            use ::sysapi::ffi::c_int;

            std::thread_local! {
                static FE_ROUND: Cell<c_int> = const { Cell::new(super::FE_TONEAREST) };
            }

            /// Returns the calling thread's rounding mode.
            pub fn get() -> c_int {
                FE_ROUND.with(Cell::get)
            }

            /// Sets the calling thread's rounding mode.
            pub fn set(mode: c_int) {
                FE_ROUND.with(|cell| cell.set(mode));
            }
        }

        #[cfg(not(feature = "std"))]
        mod round {
            use ::core::sync::atomic::{
                AtomicI32,
                Ordering,
            };
            use ::sysapi::ffi::c_int;

            static FE_ROUND: AtomicI32 = AtomicI32::new(super::FE_TONEAREST);

            /// Returns the current rounding mode.
            pub fn get() -> c_int {
                FE_ROUND.load(Ordering::Relaxed)
            }

            /// Sets the current rounding mode.
            pub fn set(mode: c_int) {
                FE_ROUND.store(mode, Ordering::Relaxed);
            }
        }
    };
}

define_round_storage!();

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns the current rounding mode.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fegetround.html>
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fegetround() -> c_int {
    round::get()
}

/// Sets the rounding mode to `mode`.
///
/// # Returns
///
/// Zero on success, or non-zero if `mode` is not a valid rounding mode.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fesetround.html>
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fesetround(mode: c_int) -> c_int {
    match mode {
        FE_TONEAREST | FE_DOWNWARD | FE_UPWARD | FE_TOWARDZERO => {
            round::set(mode & FE_ROUND_MASK);
            0
        },
        _ => -1,
    }
}

/// Clears the specified floating-point exceptions. No-op: exceptions are not tracked.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn feclearexcept(_excepts: c_int) -> c_int {
    0
}

/// Raises the specified floating-point exceptions. No-op: exceptions are not tracked.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn feraiseexcept(_excepts: c_int) -> c_int {
    0
}

/// Tests the specified floating-point exceptions. Always reports none set.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn fetestexcept(_excepts: c_int) -> c_int {
    0
}

/// Stores the current floating-point environment in `*envp`.
///
/// # Safety
///
/// `envp` must point to a writable `fenv_t` (a `c_int`).
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fegetenv(envp: *mut fenv_t) -> c_int {
    if !envp.is_null() {
        unsafe {
            *envp = fegetround();
        }
    }
    0
}

/// Restores the floating-point environment from `*envp`.
///
/// # Safety
///
/// `envp` must point to a readable `fenv_t` (a `c_int`).
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fesetenv(envp: *const fenv_t) -> c_int {
    // FE_DFL_ENV is the sentinel (const fenv_t *)-1: reset to the default
    // environment (round to nearest) rather than dereferencing it.
    if envp == FE_DFL_ENV {
        fesetround(FE_TONEAREST);
    } else if !envp.is_null() {
        fesetround(unsafe { *envp });
    }
    0
}

/// Saves the current environment in `*envp` and clears exceptions.
///
/// # Safety
///
/// `envp` must point to a writable `fenv_t` (a `c_int`).
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn feholdexcept(envp: *mut fenv_t) -> c_int {
    fegetenv(envp)
}

/// Restores the environment saved in `*envp`.
///
/// # Safety
///
/// `envp` must point to a readable `fenv_t` (a `c_int`).
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn feupdateenv(envp: *const fenv_t) -> c_int {
    fesetenv(envp)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_rounding_mode_roundtrip() {
        // The default rounding mode is round-to-nearest.
        assert_eq!(fegetround(), FE_TONEAREST);

        for &mode in &[FE_DOWNWARD, FE_UPWARD, FE_TOWARDZERO, FE_TONEAREST] {
            assert_eq!(fesetround(mode), 0);
            assert_eq!(fegetround(), mode);
        }

        // Restore the default so other tests observe a clean environment.
        assert_eq!(fesetround(FE_TONEAREST), 0);
    }

    #[test]
    fn test_invalid_rounding_mode() {
        // An unrecognized mode is rejected and does not change the environment.
        assert_ne!(fesetround(0x1234), 0);
    }

    #[test]
    fn test_exceptions_are_noops() {
        assert_eq!(feclearexcept(0x3f), 0);
        assert_eq!(feraiseexcept(0x3f), 0);
        assert_eq!(fetestexcept(0x3f), 0);
    }
}
