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
    sys_types::{
        gid_t,
        uid_t,
    },
};
use ::core::mem;

//==================================================================================================
// Types
//==================================================================================================

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct passwd {
    /// Username.
    pub pw_name: *const c_char,
    /// Encrypted password.
    pub pw_passwd: *const c_char,
    /// User ID.
    pub pw_uid: uid_t,
    /// Group ID.
    pub pw_gid: gid_t,
    /// User ID of the user who created this entry.
    pub pw_gecos: *const c_char,
    /// Home directory.
    pub pw_dir: *const c_char,
    /// Login shell.
    pub pw_shell: *const c_char,
}

//==================================================================================================
// Layout Assertions
//==================================================================================================

// `passwd` uses `#[repr(C)]`, not `#[repr(C, packed)]`, so the value behind the `*mut passwd`
// returned by `getpwuid()` is naturally aligned for C callers; a packed (alignment-1) layout
// would make those pointer-sized loads undefined behavior.
// `passwd` is an FFI type shared with C consumers (e.g. newlib's `getpwuid`). `#[repr(C)]` makes
// Rust follow the platform C ABI layout rules for the *current* target, so the concrete offsets
// differ between x86 (32-bit pointers) and x86_64 (64-bit pointers) while always matching that
// target's C compiler. The assertions below are written in terms of `size_of`/`align_of` rather
// than hardcoded byte counts, so they hold on both targets and fail the build if a field is ever
// reordered, retyped, or has padding introduced. `pw_uid` and `pw_gid` are `c_uint` (4 bytes);
// the five remaining fields are pointers, so the layout is fully pointer-packed with no padding.
::static_assert::assert_eq_size!(
    passwd,
    5 * mem::size_of::<*const c_char>() + mem::size_of::<uid_t>() + mem::size_of::<gid_t>()
);
::static_assert::assert_eq_align!(passwd, mem::align_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(passwd, pw_name) == 0);
::static_assert::assert_eq!(mem::offset_of!(passwd, pw_passwd) == mem::size_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(passwd, pw_uid) == 2 * mem::size_of::<*const c_char>());
::static_assert::assert_eq!(
    mem::offset_of!(passwd, pw_gid)
        == 2 * mem::size_of::<*const c_char>() + mem::size_of::<uid_t>()
);
::static_assert::assert_eq!(
    mem::offset_of!(passwd, pw_gecos)
        == 2 * mem::size_of::<*const c_char>() + mem::size_of::<uid_t>() + mem::size_of::<gid_t>()
);
::static_assert::assert_eq!(
    mem::offset_of!(passwd, pw_dir)
        == 3 * mem::size_of::<*const c_char>() + mem::size_of::<uid_t>() + mem::size_of::<gid_t>()
);
::static_assert::assert_eq!(
    mem::offset_of!(passwd, pw_shell)
        == 4 * mem::size_of::<*const c_char>() + mem::size_of::<uid_t>() + mem::size_of::<gid_t>()
);
