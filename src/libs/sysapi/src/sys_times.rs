// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys_types::clock_t;

//==================================================================================================
// Structures
//==================================================================================================

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct tms {
    /// User CPU time.
    pub tms_utime: clock_t,
    /// System CPU time.
    pub tms_stime: clock_t,
    /// User CPU time of terminated child processes.
    pub tms_cutime: clock_t,
    /// System CPU time of terminated child processes.
    pub tms_cstime: clock_t,
}
