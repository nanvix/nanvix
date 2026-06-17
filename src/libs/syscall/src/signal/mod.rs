// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(feature = "syscall")]
pub mod bindings;

//==================================================================================================
// Imports
//==================================================================================================

use core::ffi::{
    c_int,
    c_void,
};

//==================================================================================================
// Structures
//==================================================================================================

/// Signal set type: a 64-bit blocked-signal bitmask, matching the `<signal.h>` ABI.
pub type sigset_t = u64;

/// Signal action structure, matching the `struct sigaction` ABI declared in `<signal.h>` and used
/// by the `libc_signal` crate that calls this `sigaction` binding.
#[repr(C)]
pub struct sigaction_t {
    /// Signal handler, represented as a pointer-sized value so the `<signal.h>` sentinels
    /// (`SIG_DFL`/`SIG_IGN`/`SIG_ERR`) are representable without forming invalid function pointers.
    pub sa_handler: usize,
    pub sa_mask: sigset_t,
    pub sa_flags: c_int,
    pub sa_sigaction: Option<unsafe extern "C" fn(c_int, *mut c_void, *mut c_void)>,
}
