// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::size_t,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Returns when a directory cannot be read.
pub const GLOB_ERR: c_int = 0x0001;
/// Appends a slash to each matched directory.
pub const GLOB_MARK: c_int = 0x0002;
/// Leaves matched pathnames unsorted.
pub const GLOB_NOSORT: c_int = 0x0004;
/// Reserves leading slots in the result vector.
pub const GLOB_DOOFFS: c_int = 0x0008;
/// Returns the pattern itself when no pathname matches.
pub const GLOB_NOCHECK: c_int = 0x0010;
/// Appends matches to an existing result vector.
pub const GLOB_APPEND: c_int = 0x0020;
/// Disables backslash escaping in the pattern.
pub const GLOB_NOESCAPE: c_int = 0x0040;

/// Indicates that memory allocation failed.
pub const GLOB_NOSPACE: c_int = 1;
/// Indicates that a directory scan was aborted.
pub const GLOB_ABORTED: c_int = 2;
/// Indicates that no pathname matched the pattern.
pub const GLOB_NOMATCH: c_int = 3;
/// Indicates that pathname globbing is not implemented.
pub const GLOB_NOSYS: c_int = 4;

//==================================================================================================
// Structures
//==================================================================================================

/// Result of a pathname glob operation.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct glob_t {
    /// Number of matched pathnames.
    pub gl_pathc: size_t,
    /// Null-terminated vector of matched pathnames.
    pub gl_pathv: *mut *mut c_char,
    /// Number of reserved slots at the beginning of [`glob_t::gl_pathv`].
    pub gl_offs: size_t,
}

::static_assert::assert_eq_size!(glob_t, 3 * ::core::mem::size_of::<size_t>());
::static_assert::assert_eq_align!(glob_t, ::core::mem::align_of::<size_t>());
