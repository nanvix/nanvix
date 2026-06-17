// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

// Attributes
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(core_intrinsics)]
#![allow(internal_features)]
// Lints
#![allow(clippy::approx_constant)]
#![forbid(clippy::unwrap_used)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::cast_possible_wrap)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]
// The following lints are allowed in tests to facilitate testing of error conditions.
#![cfg_attr(not(test), forbid(clippy::expect_used))]

//==================================================================================================
// Modules
//==================================================================================================

pub mod acos;
pub mod acosf;
pub mod acosh;
pub mod asin;
pub mod asinf;
pub mod asinh;
pub mod atan;
pub mod atan2;
pub mod atan2f;
pub mod atanf;
pub mod atanh;
pub mod cbrt;
pub mod cbrtf;
pub mod ceil;
pub mod ceilf;
pub mod copysign;
pub mod copysignf;
pub mod cos;
pub mod cosf;
pub mod cosh;
pub mod erf;
pub mod erfc;
pub mod exp;
pub mod exp2;
pub mod exp2f;
pub mod expf;
pub mod expm1;
pub mod fabs;
pub mod fabsf;
pub mod fenv;
pub mod floor;
pub mod floorf;
pub mod fma;
pub mod fmaf;
pub mod fmax;
pub mod fmaxf;
pub mod fmin;
pub mod fminf;
pub mod fmod;
pub mod fmodf;
pub mod fpclassify;
pub mod frexp;
pub mod frexpf;
pub mod gamma;
pub mod hypot;
pub mod hypotf;
pub mod isinf;
pub mod isnan;
pub mod ldexp;
pub mod ldexpf;
pub mod lgamma;
pub mod log;
pub mod log10;
pub mod log10f;
pub mod log1p;
pub mod log2;
pub mod log2f;
pub mod logf;
pub mod lrint;
pub mod modf;
pub mod modff;
pub mod nextafter;
pub mod pow;
pub mod powf;
pub mod remainder;
pub mod round;
pub mod roundf;
pub mod scalbn;
pub mod scalbnf;
pub mod signbit;
pub mod sin;
pub mod sinf;
pub mod sinh;
pub mod sqrt;
pub mod sqrtf;
pub mod tan;
pub mod tanf;
pub mod tanh;
pub mod tgamma;
pub mod trunc;
pub mod truncf;
