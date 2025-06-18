// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Constants
//==================================================================================================

/// Socket options values to be used with `setsockopt()` and `getsockopt()`.
pub mod socket_option_values {
    use crate::ffi::c_int;

    /// Don't delay send to coalesce packets.
    pub const TCP_NODELAY: c_int = 1;
    /// N, time to establish connection.
    pub const TCP_KEEPINIT: c_int = 128;
    /// L,N,X start keeplives after this period.
    pub const TCP_KEEPIDLE: c_int = 256;
    /// L,N interval between keepalives.
    pub const TCP_KEEPINTVL: c_int = 512;
    /// L,N number of keepalives before close.
    pub const TCP_KEEPCNT: c_int = 1024;
}
