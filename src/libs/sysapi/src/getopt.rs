// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Indicates that a long option takes no argument.
pub const NO_ARGUMENT: c_int = 0;
/// Indicates that a long option requires an argument.
pub const REQUIRED_ARGUMENT: c_int = 1;
/// Indicates that a long option takes an optional argument.
pub const OPTIONAL_ARGUMENT: c_int = 2;

//==================================================================================================
// Structures
//==================================================================================================

/// Describes one long option accepted by `getopt_long()`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct option {
    /// Long option name without the leading `--`.
    pub name: *const c_char,
    /// One of [`NO_ARGUMENT`], [`REQUIRED_ARGUMENT`], or [`OPTIONAL_ARGUMENT`].
    pub has_arg: c_int,
    /// Optional location that receives [`option::val`].
    pub flag: *mut c_int,
    /// Value returned or stored when the option is matched.
    pub val: c_int,
}
