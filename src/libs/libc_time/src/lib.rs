// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

// Attributes
#![cfg_attr(not(feature = "std"), no_std)]
// Lints
#![forbid(clippy::unwrap_used)]
// Time arithmetic requires width conversions (i64 → i32) for struct tm fields.
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::cast_possible_wrap)]
// difftime() returns f64 from i64 subtraction, as required by the C standard.
#![deny(clippy::cast_precision_loss)]
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

pub mod asctime;
pub mod asctime_r;
pub mod clock;
pub mod ctime;
pub mod ctime_r;
pub mod difftime;
pub mod gmtime;
pub mod gmtime_r;
pub mod localtime;
pub mod localtime_r;
pub mod mktime;
pub mod time;
pub mod tm_struct;
