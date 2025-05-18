// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::{
    String,
    ToString,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the name of the current host.
///
/// # Returns
///
/// The name of the current host.
///
pub fn gethostname() -> String {
    ::syslog::trace!("gethostname()");
    env!("NANVIX_NODENAME").to_string()
}
