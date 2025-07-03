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
        c_int,
        c_long,
        c_void,
    },
    sys_types::pid_t,
};

//==================================================================================================
// Types
//==================================================================================================

/// Signal set type
pub type sigset_t = c_long;

//==================================================================================================
// Structures
//==================================================================================================

/// Signal information structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct siginfo_t {
    /// Signal number
    pub si_signo: c_int,
    /// Signal code
    pub si_code: c_int,
    /// Signal value
    pub si_value: c_int,
    /// Sending process ID
    pub si_pid: pid_t,
    /// Real user ID of sending process
    pub si_uid: c_int,
    /// Exit value or signal
    pub si_status: c_int,
    /// User time consumed
    pub si_utime: c_long,
    /// System time consumed
    pub si_stime: c_long,
    /// Address of faulting instruction
    pub si_addr: *mut c_void,
    /// Band event for SIGPOLL
    pub si_band: c_long,
    /// File descriptor
    pub si_fd: c_int,
}

/// Signal action structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct sigaction {
    /// Signal handler function
    pub sa_handler: Option<unsafe extern "C" fn(c_int)>,
    /// Extended signal handler function
    pub sa_sigaction: Option<unsafe extern "C" fn(c_int, *mut siginfo_t, *mut c_void)>,
    /// Additional signals to be blocked
    pub sa_mask: sigset_t,
    /// Special flags
    pub sa_flags: c_int,
    /// Signal restorer function (not portable)
    pub sa_restorer: Option<unsafe extern "C" fn()>,
}

//==================================================================================================
// Constants
//==================================================================================================

/// Default signal disposition
pub const SIG_DFL: usize = 0;

/// Ignore signal
pub const SIG_IGN: usize = 1;

/// Signal handler uses siginfo
pub const SA_SIGINFO: c_int = 0x00000004;

/// Don't add signal to mask
pub const SA_NODEFER: c_int = 0x40000000u32 as c_int;

/// Restore signal handler to default
pub const SA_RESETHAND: c_int = 0x80000000u32 as c_int;

/// Restart system calls
pub const SA_RESTART: c_int = 0x10000000;

/// Don't receive SIGCHLD when children stop
pub const SA_NOCLDSTOP: c_int = 0x00000001;

/// Don't create zombies
pub const SA_NOCLDWAIT: c_int = 0x00000002;

/// Use alternate stack
pub const SA_ONSTACK: c_int = 0x08000000; 