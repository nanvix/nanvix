// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use crate::sys::select::*;

//==================================================================================================
// Re-Exports
//==================================================================================================

// TODO: import `fd_set`` type and the `timeval` structure from `sys::select`.

pub use crate::sys::types::time_t;

pub use crate::sys::types::suseconds_t;

// TODO: import  `FD_CLR()`, `FD_ISSET()`, `FD_SET()`, `FD_ZERO()`, functions and  FD_SETSIZE constant from `sys::select`.

// TODO: import all symbols from `sys::select` visible.
