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
    c_int,
    c_ulong,
};

//==================================================================================================
// Constants
//==================================================================================================

/// CPU time limit, in seconds.
pub const RLIMIT_CPU: c_int = 0;
/// Maximum file size limit.
pub const RLIMIT_FSIZE: c_int = 1;
/// Maximum data segment size limit.
pub const RLIMIT_DATA: c_int = 2;
/// Maximum stack size limit.
pub const RLIMIT_STACK: c_int = 3;
/// Maximum core file size limit.
pub const RLIMIT_CORE: c_int = 4;
/// Maximum resident set size limit.
pub const RLIMIT_RSS: c_int = 5;
/// Maximum number of processes limit.
pub const RLIMIT_NPROC: c_int = 6;
/// Maximum number of open files limit.
pub const RLIMIT_NOFILE: c_int = 7;
/// Maximum locked-in-memory size limit.
pub const RLIMIT_MEMLOCK: c_int = 8;
/// Maximum address space size limit.
pub const RLIMIT_AS: c_int = 9;
/// Number of resource limits.
pub const RLIMIT_NLIMITS: c_int = 10;

/// Unlimited resource value.
pub const RLIM_INFINITY: rlim_t = (!0usize) as c_ulong;
/// Unrepresentable saved soft-limit value.
pub const RLIM_SAVED_CUR: rlim_t = RLIM_INFINITY;
/// Unrepresentable saved hard-limit value.
pub const RLIM_SAVED_MAX: rlim_t = RLIM_INFINITY;

/// Identifies a process.
pub const PRIO_PROCESS: c_int = 0;
/// Identifies a process group.
pub const PRIO_PGRP: c_int = 1;
/// Identifies a user.
pub const PRIO_USER: c_int = 2;

//==================================================================================================
// Types
//==================================================================================================

/// Used for resource limit values.
pub type rlim_t = c_ulong;

//===================================================================================================
// Structures
//===================================================================================================

///
/// # Description
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct rlimit {
    /// Soft limit.
    pub rlim_cur: rlim_t,
    /// Hard limit.
    pub rlim_max: rlim_t,
}
::static_assert::assert_eq_size!(rlimit, rlimit::_SIZE);
::static_assert::assert_eq_align!(rlimit, core::mem::align_of::<rlim_t>());

impl rlimit {
    /// Size of `rlim_cur` field, used for static size assertions.
    pub const _RLIM_CUR_SIZE: usize = core::mem::size_of::<rlim_t>();
    /// Size of `rlim_max` field, used for static size assertions.
    pub const _RLIM_MAX_SIZE: usize = core::mem::size_of::<rlim_t>();
    /// Size of the `rlimit` structure, used for static size assertions.
    pub const _SIZE: usize = Self::_RLIM_CUR_SIZE + Self::_RLIM_MAX_SIZE;
}
