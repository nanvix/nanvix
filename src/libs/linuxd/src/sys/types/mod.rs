// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi;

//==================================================================================================

/// Used for clock ID type in the clock and timer functions.
pub type clockid_t = i32;

/// Used for file attributes.
pub type mode_t = ffi::c_uint;

/// Used for time in seconds.
pub type time_t = i64;
