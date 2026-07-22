// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::c_int;

//==================================================================================================
// Types
//==================================================================================================

/// Floating-point environment.
pub type fenv_t = c_int;

/// Floating-point exception flags.
pub type fexcept_t = c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// Round to nearest (default).
pub const FE_TONEAREST: c_int = 0x0000;
/// Round toward negative infinity.
pub const FE_DOWNWARD: c_int = 0x0400;
/// Round toward positive infinity.
pub const FE_UPWARD: c_int = 0x0800;
/// Round toward zero.
pub const FE_TOWARDZERO: c_int = 0x0c00;

/// Invalid floating-point operation.
pub const FE_INVALID: c_int = 0x01;
/// Denormalized operand.
pub const FE_DENORMAL: c_int = 0x02;
/// Division by zero.
pub const FE_DIVBYZERO: c_int = 0x04;
/// Floating-point overflow.
pub const FE_OVERFLOW: c_int = 0x08;
/// Floating-point underflow.
pub const FE_UNDERFLOW: c_int = 0x10;
/// Inexact floating-point result.
pub const FE_INEXACT: c_int = 0x20;
/// Mask of all supported floating-point exceptions.
pub const FE_ALL_EXCEPT: c_int =
    FE_INVALID | FE_DENORMAL | FE_DIVBYZERO | FE_OVERFLOW | FE_UNDERFLOW | FE_INEXACT;

/// Sentinel that requests the default floating-point environment.
pub const FE_DFL_ENV: *const c_int = -1isize as *const c_int;
