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
    ffi::c_char,
    sys_types::gid_t,
};
use ::core::mem;

//==================================================================================================
// Types
//==================================================================================================

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct group {
    /// Group name.
    pub gr_name: *const c_char,
    /// Encrypted password.
    pub gr_passwd: *const c_char,
    /// Group ID.
    pub gr_gid: gid_t,
    /// Pointer to a null-terminated array of pointers to member names.
    pub gr_mem: *const *const c_char,
}

//==================================================================================================
// Layout Assertions
//==================================================================================================

// `group` is an FFI type shared with C consumers. `#[repr(C)]` makes Rust follow the platform C ABI
// layout rules for the *current* target, so the concrete offsets differ between x86 (32-bit
// pointers) and x86_64 (64-bit pointers) while always matching that target's C compiler. The
// assertions below are written in terms of `size_of`/`align_of` rather than hardcoded byte counts,
// so they hold on both targets and fail the build if a field is ever reordered or retyped.
//
// `gr_gid` is a `c_uint` (4 bytes) sitting between two pointers. Because it is no wider than a
// pointer, the following pointer (`gr_mem`) starts at the next pointer-aligned offset, which equals
// `3 * size_of::<pointer>()` on both 32-bit and 64-bit targets. The structure is therefore
// equivalent in size to four pointers.
::static_assert::assert_eq_size!(group, 4 * mem::size_of::<*const c_char>());
::static_assert::assert_eq_align!(group, mem::align_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(group, gr_name) == 0);
::static_assert::assert_eq!(mem::offset_of!(group, gr_passwd) == mem::size_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(group, gr_gid) == 2 * mem::size_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(group, gr_mem) == 3 * mem::size_of::<*const c_char>());
