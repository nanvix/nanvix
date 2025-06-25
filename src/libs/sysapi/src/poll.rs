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
    c_short,
    c_uint,
};

//==================================================================================================
// Types
//==================================================================================================

// Used for the number of file descriptors.
pub type nfds_t = c_uint;

//==================================================================================================
// Constants
//==================================================================================================

/// Events that can be polled for.
pub mod poll_flags {
    use crate::ffi::c_short;

    /// Data other than high-priority data may be read without blocking.
    pub const POLLIN: c_short = 0x0001;
    /// High priority data may be read without blocking.
    pub const POLLPRI: c_short = 0x0002;
    /// Normal data may be written without blocking.
    pub const POLLOUT: c_short = 0x0004;
    /// Normal data may be read without blocking.
    pub const POLLRDNORM: c_short = 0x0040;
    /// Priority data may be read without blocking.
    pub const POLLRDBAND: c_short = 0x0080;
    /// Normal data may be written without blocking (equivalent to POLLOUT.).
    pub const POLLWRNORM: c_short = POLLOUT;
    /// High-priority data may be written without blocking.
    pub const POLLWRBAND: c_short = 0x0100;
}

/// Errors that can occur during polling.
pub mod poll_errors {
    use crate::ffi::c_short;

    /// An error has occurred (revents only).
    pub const POLLERR: c_short = 0x0008;
    /// Device has been disconnected (revents only).
    pub const POLLHUP: c_short = 0x0010;
    /// Invalid file descriptor member (revents only).
    pub const POLLNVAL: c_short = 0x0020;
}

//==================================================================================================
// Structures
//==================================================================================================

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct pollfd {
    /// The following descriptor being polled.
    pub fd: c_int,
    /// Input event flags.
    pub events: c_short,
    /// Output event flags.
    pub revents: c_short,
}
::static_assert::assert_eq_size!(pollfd, pollfd::_SIZE);

impl pollfd {
    /// Size of the `pollfd` structure.
    pub const _SIZE: usize = ::core::mem::size_of::<c_int>() + // fd
        ::core::mem::size_of::<c_short>() + // events
        ::core::mem::size_of::<c_short>(); // revents
}

//==================================================================================================
// Function Prototypes
//==================================================================================================

unsafe extern "C" {
    pub fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int;
}
