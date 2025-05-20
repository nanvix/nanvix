// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::{
    String,
    ToString,
};
use ::config::system::DEFAULT_NODE_NAME;

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
    option_env!("NANVIX_NODENAME")
        .unwrap_or(DEFAULT_NODE_NAME)
        .to_string()
}
