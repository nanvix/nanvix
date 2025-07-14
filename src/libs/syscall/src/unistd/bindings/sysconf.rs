// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_int,
    c_long,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[unsafe(no_mangle)]
pub extern "C" fn sysconf(name: c_int) -> c_long {
    ::syslog::trace!("sysconf(): name={name:?}");
    // TODO: https://github.com/nanvix/nanvix/issues/342
    ::syslog::error!("sysconf(): not implemented");
    0
}
