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

//==================================================================================================
// Types
//==================================================================================================

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
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
